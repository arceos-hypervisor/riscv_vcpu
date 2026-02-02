use axerrno::{AxError, AxResult};

use riscv::register::sie;
use riscv::register::stvec;
use riscv_h::register::{hedeleg, hideleg, hvip};

use crate::consts::traps;
use crate::has_hardware_support;

/// Risc-V per-CPU state.
#[repr(C)]
#[repr(align(4096))]
pub struct RISCVPerCpu {
    ori_stvec: usize,
}

impl RISCVPerCpu {
    pub fn new() -> AxResult<Self> {
        let ori_stvec = unsafe { stvec::read().bits() };

        Ok(Self {
            ori_stvec,
        })
    }

    fn is_enabled(&self) -> bool {
        has_hardware_support()
    }

    pub fn hardware_enable(&mut self) -> AxResult<()> {
        if !has_hardware_support() {
            return Err(AxError::Unsupported);
        }
        unsafe {
            setup_csrs();
        }
        Ok(())
    }

    fn hardware_disable(&mut self) -> AxResult<()> {
        unimplemented!()
        // Restore original stvec.
        // unsafe {
        //     stvec::write(stvec::Stvec::from_bits(self.ori_stvec));
        // }
        // Ok(())
    }

    pub fn max_guest_page_table_levels(&self) -> usize {
        crate::vcpu::max_gpt_level()
    }

    pub fn pa_bits(&self) -> usize {
        crate::vcpu::pa_bits()
    }

    pub fn pa_range(&self) -> core::ops::Range<usize> {
        let pa_bits = crate::vcpu::pa_bits();
        0..(1 << pa_bits)
    }
}

/// Initialize (H)S-level CSRs to a reasonable state.
unsafe fn setup_csrs() {
    unsafe {
        // Delegate some synchronous exceptions.
        hedeleg::Hedeleg::from_bits(
            traps::exception::INST_ADDR_MISALIGN
                | traps::exception::BREAKPOINT
                | traps::exception::ENV_CALL_FROM_U_OR_VU
                | traps::exception::INST_PAGE_FAULT
                | traps::exception::LOAD_PAGE_FAULT
                | traps::exception::STORE_PAGE_FAULT
                | traps::exception::ILLEGAL_INST,
        )
        .write();

        // Delegate all VS-mode interrupts.
        hideleg::Hideleg::from_bits(
            traps::interrupt::VIRTUAL_SUPERVISOR_TIMER
                | traps::interrupt::VIRTUAL_SUPERVISOR_EXTERNAL
                | traps::interrupt::VIRTUAL_SUPERVISOR_SOFT,
        )
        .write();

        // Clear all interrupts.
        hvip::clear_vssip();
        hvip::clear_vstip();
        hvip::clear_vseip();

        // clear all interrupts.
        // the csr num of hcounteren is 0x606, the riscv repo is error!!!
        // hcounteren::Hcounteren::from_bits(0xffff_ffff).write();
        core::arch::asm!("csrw {csr}, {rs}", csr = const 0x606, rs = in(reg) -1);

        // Configure henvcfg (CSR 0x60A) to enable extensions for VS-mode
        // Bit 63 (STCE): Enable stimecmp/vstimecmp CSR access (Sstc extension)
        // Bit 7 (CBZE):  Enable cbo.zero instruction (Zicboz extension)
        // Bit 6 (CBCFE): Enable cbo.clean/flush instructions (Zicbom extension)
        // Bit 5:4 (CBIE): Enable cbo.inval instruction (00=illegal, 01=flush, 11=inval)
        let henvcfg_val: usize = (1usize << 63) | 0xF0; // STCE | CBZE | CBCFE | CBIE
        core::arch::asm!("csrw {csr}, {rs}", csr = const 0x60A, rs = in(reg) henvcfg_val);

        // enable interrupt
        // Note: With Sstc enabled, guest timer is handled by vstimecmp.
        // Don't enable sie.stimer here - it's for HOST timer only.
        sie::set_sext();
        sie::set_ssoft();
        // sie::set_stimer(); // Not needed with Sstc - guest manages its own timer via vstimecmp
    }
}
