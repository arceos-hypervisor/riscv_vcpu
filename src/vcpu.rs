use riscv::register::{scause, sie, sstatus};
use riscv_decode::{
    Instruction,
    types::{IType, SType},
};
use riscv_h::register::{
    hstatus, htimedelta, hvip,
    vsatp::{self, Vsatp},
    vscause::{self, Vscause},
    vsepc,
    vsie::{self, Vsie},
    vsscratch,
    vsstatus::{self, Vsstatus},
    vstval,
    vstvec::{self, Vstvec},
};
use rustsbi::{Forward, RustSBI};
use sbi_spec::{hsm, legacy, srst};

use crate::{
    EID_HVC, RISCVVCpuCreateConfig, consts::traps::irq::S_EXT, guest_mem, inject_interrupt, regs::*, sbi_console::*,
};

// Use axaddrspace types internally
use axaddrspace::{
    GuestPhysAddr as InnerGuestPhysAddr,
    GuestVirtAddr as InnerGuestVirtAddr,
    HostPhysAddr as InnerHostPhysAddr,
    MappingFlags,
    device::AccessWidth
};
// Use axvm_types for public API
use axvm_types::addr::{GuestPhysAddr, HostPhysAddr};

use axerrno::{AxError::InvalidData, AxResult};
use axvcpu::AxVCpuExitReason;

unsafe extern "C" {
    fn _run_guest(state: *mut VmCpuRegisters);
}

const TINST_PSEUDO_STORE: u32 = 0x3020;
const TINST_PSEUDO_LOAD: u32 = 0x3000;

#[inline]
fn instr_is_pseudo(ins: u32) -> bool {
    ins == TINST_PSEUDO_STORE || ins == TINST_PSEUDO_LOAD
}

#[repr(C)]
#[derive(Default, Debug)]
/// A virtual CPU within a guest
pub struct RISCVVCpu {
    regs: VmCpuRegisters,
    sbi: RISCVVCpuSbi,
    pub pt_level: usize,
    pub pa_bits: usize,
}

#[derive(RustSBI, Debug)]
struct RISCVVCpuSbi {
    #[rustsbi(console, pmu, fence, reset, info, hsm)]
    forward: Forward,
}

impl Default for RISCVVCpuSbi {
    #[inline]
    fn default() -> Self {  
        Self { forward: Forward }
    }
}

impl RISCVVCpu {
    pub fn new(config: RISCVVCpuCreateConfig) -> AxResult<Self> {
        let mut regs = VmCpuRegisters::default();
        // Setup the guest's general purpose registers.
        // `a0` is the hartid
        regs.guest_regs.gprs.set_reg(GprIndex::A0, config.hart_id);
        // `a1` is the address of the device tree blob.
        regs.guest_regs.gprs.set_reg(GprIndex::A1, config.dtb_addr);

        let pa_bits = pa_bits();
        let pt_level = max_gpt_level();

        Ok(Self {
            regs,
            sbi: RISCVVCpuSbi::default(),
            pt_level,
            pa_bits,
        })
    }

    pub fn setup(&mut self) -> AxResult {
        // Set sstatus.
        let mut sstatus = sstatus::read();
        sstatus.set_sie(false);
        sstatus.set_spie(false);
        sstatus.set_spp(sstatus::SPP::Supervisor);
        self.regs.guest_regs.sstatus = sstatus.bits();

        // Set hstatus.
        let mut hstatus = hstatus::read();
        hstatus.set_spv(true);
        hstatus.set_vsxl(hstatus::VsxlValues::Vsxl64);
        // Set SPVP bit in order to accessing VS-mode memory from HS-mode.
        hstatus.set_spvp(true);
        // Note: Do NOT set VTW (Virtual Trap WFI) here. While VTW makes WFI trap,
        // it causes a busy-loop trap storm (~40K traps/sec) that wastes CPU.
        // Instead, we use a periodic watchdog timer to force VM exits for console polling.
        self.regs.guest_regs.hstatus = hstatus.bits();
        Ok(())
    }

    pub fn set_entry(&mut self, entry: GuestPhysAddr) -> AxResult {
        debug!("vCPU set entry address: {entry:#?}");
        // Convert from axvm_types::addr::GuestPhysAddr to internal representation
        self.regs.guest_regs.sepc = entry.as_usize();
        Ok(())
    }

    pub fn set_dtb_addr(&mut self, dtb_addr: GuestPhysAddr) -> AxResult {
        debug!("vCPU set DTB address: {dtb_addr:#?}");
        // On RISC-V, the DTB address is passed in a1 register
        self.regs.guest_regs.gprs.set_reg(GprIndex::A1, dtb_addr.as_usize());
        Ok(())
    }

    pub fn set_hart_id(&mut self, hart_id: usize) -> AxResult {
        debug!("vCPU set hart ID: {hart_id:#x}");
        // On RISC-V, the hart ID is passed in a0 register
        self.regs.guest_regs.gprs.set_reg(GprIndex::A0, hart_id);
        Ok(())
    }

    pub fn set_ept_root(&mut self, ept_root: HostPhysAddr) -> AxResult {
        debug!("set_ept_root: ept_root={:#x}", ept_root.as_usize());
        // hgatp format: [63:60] MODE, [57:44] VMID, [43:0] PPN
        // MODE: Sv39x4=8, Sv48x4=9, Sv57x4=10
        let mode = match self.pt_level {
            3 => 8,  // Sv39x4
            4 => 9,  // Sv48x4
            5 => 10, // Sv57x4
            _ => {
                warn!("Invalid pt_level {}, defaulting to Sv48x4", self.pt_level);
                9
            }
        };
        // Ensure only bits [43:0] are used for PPN
        let ppn = (usize::from(ept_root) >> 12) & ((1 << 44) - 1);
        let hgatp = (mode << 60) | ppn;  // VMID=0 initially
        self.regs.virtual_hs_csrs.hgatp = hgatp;
        Ok(())
    }

    pub fn setup_current_cpu(&mut self, vmid: usize) -> AxResult {
        // For RISC-V H extension, set the VMID in hgatp and flush G-stage TLB.
        //
        // hgatp format:
        // [63:60] MODE: Address translation mode (Sv39x4=8, Sv48x4=9, Sv57x4=10)
        // [59:44] VMID: Virtual machine identifier (16 bits)
        // [43:0]  PPN:  Physical page number (stage-2 page table base)

        // Extract current MODE and PPN from hgatp
        let current_hgatp = self.regs.virtual_hs_csrs.hgatp;
        let mode = (current_hgatp >> 60) & 0xF;
        let ppn = current_hgatp & ((1 << 44) - 1);

        // Reconstruct hgatp with VMID set
        // VMID is 16 bits, ensure it doesn't overflow
        let vmid = vmid & 0xFFFF;
        // Clear VMID field (bits [59:44]) and set new VMID
        let vmid_field = (vmid & 0xFFFF) << 44;
        self.regs.virtual_hs_csrs.hgatp = (mode << 60) | vmid_field | ppn;

        unsafe {
            // Load hgatp to hardware
            core::arch::asm!(
                "csrw hgatp, {hgatp}",
                hgatp = in(reg) self.regs.virtual_hs_csrs.hgatp,
            );

            // Flush all G-stage TLB entries for this VM
            // This is similar to ARM's "tlbi vmalls12e1is"
            core::arch::asm!("hfence.gvma");

            // Ensure the fence completes
            core::arch::asm!("sfence.vma");
        }

        debug!(
            "vCPU setup: vmid={}, hgatp={:#x}, mode={}, ppn={:#x}",
            vmid,
            self.regs.virtual_hs_csrs.hgatp,
            mode,
            ppn
        );

        Ok(())
    }

    pub fn run(&mut self) -> AxResult<AxVCpuExitReason> {

        unsafe {
            sstatus::clear_sie();
            sie::set_sext();
            sie::set_ssoft();
            sie::set_stimer();

            // Watchdog: set host stimecmp to fire in ~10ms (10MHz timer)
            // This forces periodic VM exits so the hypervisor can poll console input.
            let current_time: u64;
            core::arch::asm!("rdtime {t}", t = out(reg) current_time);
            let stimecmp = current_time + 100_000;
            core::arch::asm!("csrw {csr}, {val}", csr = const 0x14D, val = in(reg) stimecmp);
        }

        unsafe {
            _run_guest(&mut self.regs);
        }

        unsafe {
            sie::clear_sext();
            sie::clear_ssoft();
            sie::clear_stimer();
            sstatus::set_sie();
        }
        self.vmexit_handler()
    }

    pub fn bind(&mut self) -> AxResult {
        // Load the vCPU's CSRs from the stored state.
        unsafe {
            let vsatp = Vsatp::from_bits(self.regs.vs_csrs.vsatp);
            vsatp.write();
            let vstvec = Vstvec::from_bits(self.regs.vs_csrs.vstvec);
            vstvec.write();
            let vsepc = self.regs.vs_csrs.vsepc;
            vsepc::write(vsepc);
            let vstval = self.regs.vs_csrs.vstval;
            vstval::write(vstval);
            let htimedelta = self.regs.vs_csrs.htimedelta;
            htimedelta::write(htimedelta);
            let vscause = Vscause::from_bits(self.regs.vs_csrs.vscause);
            vscause.write();
            let vsscratch = self.regs.vs_csrs.vsscratch;
            vsscratch::write(vsscratch);
            let vsstatus = Vsstatus::from_bits(self.regs.vs_csrs.vsstatus);
            vsstatus.write();
            let vsie = Vsie::from_bits(self.regs.vs_csrs.vsie);
            vsie.write();
            core::arch::asm!(
                "csrw hgatp, {hgatp}",
                hgatp = in(reg) self.regs.virtual_hs_csrs.hgatp,
            );
            core::arch::riscv64::hfence_gvma_all();
        }
        Ok(())
    }

    pub fn unbind(&mut self) -> AxResult {
        // Store the vCPU's CSRs to the stored state.
        unsafe {
            self.regs.vs_csrs.vsatp = vsatp::read().bits();
            self.regs.vs_csrs.vstvec = vstvec::read().bits();
            self.regs.vs_csrs.vsepc = vsepc::read();
            self.regs.vs_csrs.vstval = vstval::read();
            self.regs.vs_csrs.htimedelta = htimedelta::read();
            self.regs.vs_csrs.vscause = vscause::read().bits();
            self.regs.vs_csrs.vsscratch = vsscratch::read();
            self.regs.vs_csrs.vsstatus = vsstatus::read().bits();
            self.regs.vs_csrs.vsie = vsie::read().bits();
            core::arch::asm!(
                "csrr {hgatp}, hgatp",
                hgatp = out(reg) self.regs.virtual_hs_csrs.hgatp,
            );
            core::arch::asm!("csrw hgatp, x0");
            core::arch::riscv64::hfence_gvma_all();
        }
        Ok(())
    }

    /// Set one of the vCPU's general purpose register.
    pub fn set_gpr(&mut self, index: usize, val: usize) {
        match index {
            0 => {
                // Do nothing, x0 is hardwired to zero
            }
            1..=31 => {
                if let Some(gpr_index) = GprIndex::from_raw(index as u32) {
                    self.set_gpr_from_gpr_index(gpr_index, val);
                } else {
                    warn!(
                        "RISCVVCpu: Failed to map general purpose register index: {}",
                        index
                    );
                }
            }
            _ => {
                warn!(
                    "RISCVVCpu: Unsupported general purpose register index: {}",
                    index
                );
            }
        }
    }

    pub fn inject_interrupt(&mut self, vector: usize) -> AxResult {
        inject_interrupt(vector);
        Ok(())
    }

    pub fn set_return_value(&mut self, val: usize) {
        self.set_gpr_from_gpr_index(GprIndex::A0, val);
    }
}

impl RISCVVCpu {
    /// Gets one of the vCPU's general purpose registers.
    pub fn get_gpr(&self, index: GprIndex) -> usize {
        self.regs.guest_regs.gprs.reg(index)
    }

    /// Set one of the vCPU's general purpose register.
    pub fn set_gpr_from_gpr_index(&mut self, index: GprIndex, val: usize) {
        self.regs.guest_regs.gprs.set_reg(index, val);
    }

    /// Advance guest pc by `instr_len` bytes
    pub fn advance_pc(&mut self, instr_len: usize) {
        self.regs.guest_regs.sepc += instr_len
    }

    /// Gets the vCPU's registers.
    pub fn regs(&mut self) -> &mut VmCpuRegisters {
        &mut self.regs
    }
}

impl RISCVVCpu {
    fn vmexit_handler(&mut self) -> AxResult<AxVCpuExitReason> {
        // info!("[VMEXIT] ========== VMEXIT HANDLER START ==========");
        self.regs.trap_csrs.load_from_hw();

        let scause = scause::read();
        use super::trap::Exception;
        use riscv::interrupt::{Interrupt, Trap};

        trace!(
            "[VMEXIT] scause={:?} (bits={:#x}), sepc={:#x}, stval={:#x}, htinst={:#x}",
            scause.cause(),
            scause.bits(),
            self.regs.guest_regs.sepc,
            self.regs.trap_csrs.stval,
            riscv_h::register::htinst::read()
        );

        // Try to convert the raw trap cause to a standard RISC-V trap cause.
        let trap: Trap<Interrupt, Exception> = scause.cause().try_into().map_err(|_| {
            error!("Unknown trap cause: scause={:#x}", scause.bits());
            InvalidData
        })?;

        match trap {
            Trap::Exception(Exception::VirtualSupervisorEnvCall) => {
                // info!("[VMEXIT] Handling Virtual Supervisor ECall");
                let a = self.regs.guest_regs.gprs.a_regs();
                let param = [a[0], a[1], a[2], a[3], a[4], a[5]];
                let extension_id = a[7];
                let function_id = a[6];

                trace!(
                    "[VMEXIT] SBI call: eid={:#x} ('{}') fid={:#x} params=[{:#x}, {:#x}, {:#x}, {:#x}, {:#x}, {:#x}]",
                    extension_id,
                    alloc::string::String::from_utf8_lossy(&(extension_id as u32).to_be_bytes()),
                    function_id,
                    param[0], param[1], param[2], param[3], param[4], param[5]
                );
                match extension_id {
                    // Compatibility with Legacy Extensions.
                    legacy::LEGACY_SET_TIMER..=legacy::LEGACY_SHUTDOWN => match extension_id {
                        legacy::LEGACY_SET_TIMER => {
                            let timer_val = param[0] as u64;
                            unsafe {
                                core::arch::asm!(
                                    "csrw {csr}, {val}",
                                    csr = const 0x24D,  // vstimecmp
                                    val = in(reg) timer_val,
                                );
                            }
                            self.set_gpr_from_gpr_index(GprIndex::A0, 0);
                        }
                        legacy::LEGACY_CONSOLE_PUTCHAR => {
                            sbi_call_legacy_1(legacy::LEGACY_CONSOLE_PUTCHAR, param[0]);
                        }
                        legacy::LEGACY_CONSOLE_GETCHAR => {
                            let c = sbi_call_legacy_0(legacy::LEGACY_CONSOLE_GETCHAR);
                            self.set_gpr_from_gpr_index(GprIndex::A0, c);
                        }
                        legacy::LEGACY_SHUTDOWN => {
                            // info!("[VMEXIT] SBI SHUTDOWN requested");
                            // sbi_call_legacy_0(LEGACY_SHUTDOWN)
                            return Ok(AxVCpuExitReason::SystemDown);
                        }
                        _ => {
                            warn!(
                                "Unsupported SBI legacy extension id {:#x} function id {:#x}",
                                extension_id, function_id
                            );
                        }
                    },
                    // Handle HSM extension
                    hsm::EID_HSM => match function_id {
                        hsm::HART_START => {
                            let hartid = a[0];
                            let start_addr = a[1];
                            let opaque = a[2];
                            // info!("[VMEXIT] HSM HART_START: hartid={:#x}, start_addr={:#x}, opaque={:#x}", hartid, start_addr, opaque);
                            self.advance_pc(4);
                            return Ok(AxVCpuExitReason::CpuUp {
                                target_cpu: hartid as _,
                                entry_point: InnerGuestPhysAddr::from(start_addr),
                                arg: opaque as _,
                            });
                        }
                        hsm::HART_STOP => {
                            // info!("[VMEXIT] HSM HART_STOP");
                            return Ok(AxVCpuExitReason::CpuDown { _state: 0 });
                        }
                        hsm::HART_SUSPEND => {
                            // Todo: support these parameters.
                            let _suspend_type = a[0];
                            let _resume_addr = a[1];
                            let _opaque = a[2];
                            // info!("[VMEXIT] HSM HART_SUSPEND: suspend_type={:#x}", _suspend_type);
                            return Ok(AxVCpuExitReason::Halt);
                        }
                        _ => {
                            warn!("[VMEXIT] Unknown HSM function id {:#x}", function_id);
                            todo!();
                        }
                    },
                    // Handle hypercall
                    EID_HVC => {
                        // info!("[VMEXIT] HYPERCALL: nr={:#x}, args=[{:#x}, {:#x}, {:#x}, {:#x}, {:#x}, {:#x}]",
                        //     function_id, param[0], param[1], param[2], param[3], param[4], param[5]);
                        self.advance_pc(4);
                        return Ok(AxVCpuExitReason::Hypercall {
                            nr: function_id as _,
                            args: [
                                param[0] as _,
                                param[1] as _,
                                param[2] as _,
                                param[3] as _,
                                param[4] as _,
                                param[5] as _,
                            ],
                        });
                    }
                    // Debug Console Extension
                    EID_DBCN => match function_id {
                        // Write from memory region to debug console.
                        FID_CONSOLE_WRITE => {
                            let num_bytes = param[0];
                            let gpa = join_u64(param[1], param[2]);
                            // info!("[VMEXIT] DBCN WRITE: num_bytes={}, gpa={:#x}", num_bytes, gpa);

                            if num_bytes == 0 {
                                self.sbi_return(RET_SUCCESS, 0);
                                return Ok(AxVCpuExitReason::Nothing);
                            }

                            let mut buf = alloc::vec![0u8; num_bytes as usize];
                            let copied = guest_mem::copy_from_guest(
                                &mut *buf,
                                InnerGuestPhysAddr::from(gpa as usize),
                            );

                            if copied == buf.len() {
                                let ret = console_write(&buf);
                                // info!("[VMEXIT] DBCN WRITE: wrote {} bytes, error={:#x}, value={:#x}", copied, ret.error, ret.value);
                                self.sbi_return(ret.error, ret.value);
                            } else {
                                // warn!("[VMEXIT] DBCN WRITE: partial copy {} != {}", copied, buf.len());
                                self.sbi_return(RET_ERR_FAILED, 0);
                            }

                            return Ok(AxVCpuExitReason::Nothing);
                        }
                        // Read to memory region from debug console.
                        FID_CONSOLE_READ => {
                            let num_bytes = param[0];
                            let gpa = join_u64(param[1], param[2]);
                            // info!("[VMEXIT] DBCN READ: num_bytes={}, gpa={:#x}", num_bytes, gpa);

                            if num_bytes == 0 {
                                self.sbi_return(RET_SUCCESS, 0);
                                return Ok(AxVCpuExitReason::Nothing);
                            }

                            let mut buf = alloc::vec![0u8; num_bytes as usize];
                            let ret = console_read(&mut buf);

                            if ret.is_ok() && ret.value <= buf.len() {
                                let copied = guest_mem::copy_to_guest(
                                    &buf[..ret.value],
                                    InnerGuestPhysAddr::from(gpa as usize),
                                );
                                if copied == ret.value {
                                    // info!("[VMEXIT] DBCN READ: read {} bytes", copied);
                                    self.sbi_return(RET_SUCCESS, ret.value);
                                } else {
                                    // warn!("[VMEXIT] DBCN READ: partial copy {} != {}", copied, ret.value);
                                    self.sbi_return(RET_ERR_FAILED, 0);
                                }
                            } else {
                                // info!("[VMEXIT] DBCN READ: error={:#x}, value={:#x}", ret.error, ret.value);
                                self.sbi_return(ret.error, ret.value);
                            }

                            return Ok(AxVCpuExitReason::Nothing);
                        }
                        // Write a single byte to debug console.
                        FID_CONSOLE_WRITE_BYTE => {
                            let byte = (param[0] & 0xff) as u8;
                            // info!("[VMEXIT] DBCN WRITE_BYTE: byte={:#x} ('{}')", byte, byte as char);
                            print_byte(byte);
                            self.sbi_return(RET_SUCCESS, 0);
                            return Ok(AxVCpuExitReason::Nothing);
                        }
                        // Unknown FID.
                        _ => {
                            // warn!("[VMEXIT] Unknown DBCN function id {:#x}", function_id);
                            self.sbi_return(RET_ERR_NOT_SUPPORTED, 0);
                            return Ok(AxVCpuExitReason::Nothing);
                        }
                    },
                    srst::EID_SRST => match function_id {
                        srst::SYSTEM_RESET => {
                            let reset_type = param[0];
                            // info!("[VMEXIT] SRST SYSTEM_RESET: reset_type={:#x}", reset_type);
                            if reset_type == srst::RESET_TYPE_SHUTDOWN as _ {
                                // Shutdown the system.
                                return Ok(AxVCpuExitReason::SystemDown);
                            } else {
                                unimplemented!("Unsupported reset type {}", reset_type);
                            }
                        }
                        _ => {
                            warn!("[VMEXIT] Unknown SRST function id {:#x}", function_id);
                            self.sbi_return(RET_ERR_NOT_SUPPORTED, 0);
                            return Ok(AxVCpuExitReason::Nothing);
                        }
                    },
                    // By default, forward the SBI call to the RustSBI implementation.
                    // See [`RISCVVCpuSbi`].
                    _ => {
                        // info!("[VMEXIT] Forwarding SBI call: eid={:#x}, fid={:#x}", extension_id, function_id);
                        let ret = self.sbi.handle_ecall(extension_id, function_id, param);
                        if ret.is_err() {
                            trace!(
                                "[VMEXIT] Forward ecall failed: eid={:#x} fid={:#x} param={:#x?} error={:#x} value={:#x}",
                                extension_id, function_id, param, ret.error, ret.value
                            );
                        }
                        self.set_gpr_from_gpr_index(GprIndex::A0, ret.error);
                        self.set_gpr_from_gpr_index(GprIndex::A1, ret.value);
                    }
                };

                // info!("[VMEXIT] SBI call completed, advancing PC by 4");
                self.advance_pc(4);
                Ok(AxVCpuExitReason::Nothing)
            }
            Trap::Interrupt(Interrupt::SupervisorTimer) => {
                // Watchdog timer handler - re-arm for next check
                unsafe {
                    let current_time: u64;
                    core::arch::asm!("rdtime {t}", t = out(reg) current_time);
                    let stimecmp = current_time + 100_000; // 10ms
                    core::arch::asm!("csrw {csr}, {val}", csr = const 0x14D, val = in(reg) stimecmp);
                }

                static LAST_SEPC: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);
                static STUCK_COUNT: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);

                let sepc = self.regs.guest_regs.sepc;
                let hip: usize;
                let vsstatus_val: usize;
                let vsie_val: usize;
                unsafe {
                    core::arch::asm!("csrr {t}, {csr}", t = out(reg) hip, csr = const 0x644);
                    core::arch::asm!("csrr {t}, {csr}", t = out(reg) vsstatus_val, csr = const 0x200);
                    core::arch::asm!("csrr {t}, {csr}", t = out(reg) vsie_val, csr = const 0x204);
                }

                let sie_disabled = (vsstatus_val & 0x2) == 0;
                let vseip_pending = (hip & 0x400) != 0;
                let seie_disabled = (vsie_val & 0x200) == 0;
                let last_sepc = LAST_SEPC.swap(sepc, core::sync::atomic::Ordering::Relaxed);

                // Detect stuck: same sepc, SIE disabled, VSEIP pending
                if sepc == last_sepc && (sie_disabled || seie_disabled) && vseip_pending {
                    let stuck = STUCK_COUNT.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
                    if stuck >= 5 {
                        // Force enable SIE and SEIE to break the deadlock
                        unsafe {
                            let new_vsstatus = vsstatus_val | 0x2;
                            core::arch::asm!("csrw {csr}, {val}", csr = const 0x200, val = in(reg) new_vsstatus);
                            let new_vsie = vsie_val | 0x200;
                            core::arch::asm!("csrw {csr}, {val}", csr = const 0x204, val = in(reg) new_vsie);
                        }
                        STUCK_COUNT.store(0, core::sync::atomic::Ordering::Relaxed);
                    }
                } else {
                    STUCK_COUNT.store(0, core::sync::atomic::Ordering::Relaxed);
                }

                Ok(AxVCpuExitReason::Nothing)
                // Ok(AxVCpuExitReason::TimerTick)
            }
            Trap::Interrupt(Interrupt::SupervisorExternal) => {
                // 9 == Interrupt::SupervisorExternal
                //
                // It's a great fault in the `riscv` crate that `Interrupt` and `Exception` are not
                // explicitly numbered, and they provide no way to convert them to a number. Also,
                // `as usize` will give use a wrong value.
                Ok(AxVCpuExitReason::ExternalInterrupt { vector: S_EXT as _ })
            }
            Trap::Exception(
                gpf @ (Exception::LoadGuestPageFault | Exception::StoreGuestPageFault),
            ) => self.handle_guest_page_fault(gpf == Exception::StoreGuestPageFault),
            _ => {
                panic!(
                    "Unhandled trap: {:?}, sepc: {:#x}, stval: {:#x}",
                    scause.cause(),
                    self.regs.guest_regs.sepc,
                    self.regs.trap_csrs.stval
                );
            }
        }
    }

    #[inline]
    fn sbi_return(&mut self, a0: usize, a1: usize) {
        self.set_gpr_from_gpr_index(GprIndex::A0, a0);
        self.set_gpr_from_gpr_index(GprIndex::A1, a1);
        self.advance_pc(4);
    }

    /// Decode the instruction at the given virtual address. Return the decoded instruction and its
    /// length in bytes.
    ///
    /// `htinst_val` is the value of htinst CSR read at the beginning of guest page fault handling.
    /// We pass it here instead of reading htinst again because htinst may be cleared between reads.
    fn decode_instr_at(&self, vaddr: InnerGuestVirtAddr, htinst_val: usize) -> AxResult<(Instruction, usize)> {
        let mut instr = htinst_val;
        let mut instr_len = 0;
        if instr == 0 {
            // htinst is 0, which may happen in some cases:
            // 1. QEMU may not always fill htinst
            // 2. Page table walk failed on the first stage
            //
            // WARNING: fetch_guest_instruction uses hlvx which will trigger a page fault
            // if the guest VA is not properly mapped. Since extable handling is not
            // implemented, this will cause a panic.
            //
            // For now, we log a warning and try to fetch anyway. In the future, we should
            // either implement extable handling or use a safer approach.
            warn!("[GPF] htinst=0, attempting to fetch instruction from guest VA {:#x}", vaddr.as_usize());

            // Read the instruction from guest memory.
            instr = guest_mem::fetch_guest_instruction(vaddr) as _;
            instr_len = riscv_decode::instruction_length(instr as u16);
            instr = match instr_len {
                2 => instr & 0xffff,
                4 => instr,
                _ => unreachable!("Unsupported instruction length: {}", instr_len),
            };
        } else if instr_is_pseudo(instr as u32) {
            error!("fault on 1st stage page table walk");
            return Err(axerrno::ax_err_type!(
                Unsupported,
                "risc-v vcpu guest page fault handler encountered pseudo instruction"
            ));
        } else {
            // Transform htinst value to standard instruction.
            // According to RISC-V Spec:
            //      Bits 1:0 of a transformed standard instruction will be binary 01 if
            //      the trapping instruction is compressed and 11 if not.
            instr_len = match (instr as u16) & 0x3 {
                0x1 => 2,
                0x3 => 4,
                _ => unreachable!("Unsupported instruction length"),
            };
            instr |= 0x2;
        }

        riscv_decode::decode(instr as u32)
            .map_err(|_| {
                axerrno::ax_err_type!(
                    Unsupported,
                    "risc-v vcpu guest pf handler decoding instruction failed"
                )
            })
            .map(|instr| (instr, instr_len))
    }

    /// Handle a guest page fault. Return an exit reason.
    fn handle_guest_page_fault(&mut self, _writing: bool) -> AxResult<AxVCpuExitReason> {
        let fault_addr = self.regs.trap_csrs.gpt_page_fault_addr();
        let sepc = self.regs.guest_regs.sepc;
        let sepc_vaddr = InnerGuestVirtAddr::from(sepc);
        let htinst = riscv_h::register::htinst::read();

        /// Temporary enum to represent the decoded operation.
        enum DecodedOp {
            Read {
                i: IType,
                width: AccessWidth,
                signed_ext: bool,
            },
            Write {
                s: SType,
                width: AccessWidth,
            },
        }

        use DecodedOp::*;

        let (decoded_instr, instr_len) = self.decode_instr_at(sepc_vaddr, htinst)?;
        let op = match decoded_instr {
            Instruction::Lb(i) => Read {
                i,
                width: AccessWidth::Byte,
                signed_ext: true,
            },
            Instruction::Lh(i) => Read {
                i,
                width: AccessWidth::Word,
                signed_ext: true,
            },
            Instruction::Lw(i) => Read {
                i,
                width: AccessWidth::Dword,
                signed_ext: true,
            },
            Instruction::Ld(i) => Read {
                i,
                width: AccessWidth::Qword,
                signed_ext: true,
            },
            Instruction::Lbu(i) => Read {
                i,
                width: AccessWidth::Byte,
                signed_ext: false,
            },
            Instruction::Lhu(i) => Read {
                i,
                width: AccessWidth::Word,
                signed_ext: false,
            },
            Instruction::Lwu(i) => Read {
                i,
                width: AccessWidth::Dword,
                signed_ext: false,
            },
            Instruction::Sb(s) => Write {
                s,
                width: AccessWidth::Byte,
            },
            Instruction::Sh(s) => Write {
                s,
                width: AccessWidth::Word,
            },
            Instruction::Sw(s) => Write {
                s,
                width: AccessWidth::Dword,
            },
            Instruction::Sd(s) => Write {
                s,
                width: AccessWidth::Qword,
            },
            _ => {
                warn!("[GPF] Unknown instruction causing page fault: {:?}", decoded_instr);
                // Not a load or store instruction, so we cannot handle it here, return a nested page fault.
                return Ok(AxVCpuExitReason::NestedPageFault {
                    addr: fault_addr,
                    access_flags: MappingFlags::empty(),
                });
            }
        };

        // WARN: This is a temporary place to add the instruction length to the guest's sepc.
        self.advance_pc(instr_len);

        Ok(match op {
            Read {
                i,
                width,
                signed_ext,
            } => {
                AxVCpuExitReason::MmioRead {
                    addr: fault_addr,
                    width,
                    reg: i.rd() as _,
                    reg_width: AccessWidth::Qword,
                    signed_ext,
                }
            }
            Write { s, width } => {
                let source_reg = s.rs2();
                let value = self.get_gpr(unsafe {
                    // SAFETY: `source_reg` is guaranteed to be in [0, 31]
                    GprIndex::from_raw(source_reg).unwrap_unchecked()
                });

                AxVCpuExitReason::MmioWrite {
                    addr: fault_addr,
                    width,
                    data: value as _,
                }
            }
        })
    }
}

#[inline(always)]
fn sbi_call_legacy_0(eid: usize) -> usize {
    let error;
    unsafe {
        core::arch::asm!(
            "ecall",
            in("a7") eid,
            lateout("a0") error,
        );
    }
    error
}

#[inline(always)]
fn sbi_call_legacy_1(eid: usize, arg0: usize) -> usize {
    let error;
    unsafe {
        core::arch::asm!(
            "ecall",
            in("a7") eid,
            inlateout("a0") arg0 => error,
        );
    }
    error
}

pub(crate) fn pa_bits() -> usize {
    #[cfg(target_arch = "riscv64")]
    {
        use riscv::register::satp;
        match satp::read().mode() {
            satp::Mode::Sv39 => {
                // 实际实现中，Sv39 通常支持 40-44 bits PA
                // 需要从设备树或平台特定方式获取确切值
                // 这里保守返回常见的实现值
                56  // 规范值，或者用平台检测
            }
            satp::Mode::Sv48 => 56,
            satp::Mode::Sv57 => 56,
            satp::Mode::Bare => {
                56  // 或从设备树读取
            }
            mode => panic!("Unsupported satp mode: {:?}", mode),
        }
    }

    #[cfg(target_arch = "riscv32")]
    {
        34  // Sv32 规范定义
    }

    #[cfg(not(any(target_arch = "riscv64", target_arch = "riscv32")))]
    {
        48  // 默认值
    }
}

/// Returns the maximum guest page table level based on host satp mode.
/// Stage-2 page table mode must match the host's Stage-1 mode.
pub(crate) fn max_gpt_level() -> usize {
    #[cfg(target_arch = "riscv64")]
    {
        use riscv::register::satp;

        match satp::read().mode() {
            satp::Mode::Sv39 => 3,  // Sv39x4 (3-level Stage-2)
            satp::Mode::Sv48 => 4,  // Sv48x4 (4-level Stage-2)
            satp::Mode::Sv57 => 5,  // Sv57x4 (5-level Stage-2)
            satp::Mode::Bare => {
                panic!("Cannot use Stage-2 translation in Bare mode")
            }
            mode => panic!("Unsupported satp mode: {:?}", mode),
        }
    }

    #[cfg(target_arch = "riscv32")]
    {
        2  // Sv32x4 (2-level Stage-2)
    }

    #[cfg(not(any(target_arch = "riscv64", target_arch = "riscv32")))]
    {
        4  // 默认值
    }
}