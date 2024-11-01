#![no_std]
#![feature(doc_cfg)]
#![feature(naked_functions)]
#![feature(riscv_ext_intrinsics)]
#![feature(asm_const)]
#![doc = include_str!("../README.md")]

#[macro_use]
extern crate log;

mod consts;
/// The Control and Status Registers (CSRs) for a RISC-V hypervisor.
mod detect;
mod percpu;
mod regs;
mod trap;
mod vcpu;

pub use self::percpu::RISCVPerCpu;
pub use self::vcpu::RISCVVCpu;
pub use detect::detect_h_extension as has_hardware_support;

/// Low-level resource interfaces that must be implemented by the crate user.
#[crate_interface::def_interface]
pub trait HalIf {
    /// Returns the physical address of the given virtual address.
    fn virt_to_phys(vaddr: axaddrspace::HostVirtAddr) -> axaddrspace::HostPhysAddr;
}

/// Extension ID for hypercall, defined by ourselves.
/// `0x48`, `0x56`, `0x43` is "HVC" in ASCII.
///
/// Borrowed from the design of `eid_from_str` in [sbi-spec](https://github.com/rustsbi/rustsbi/blob/62ab2e498ca66cdf75ce049c9dbc2f1862874553/sbi-spec/src/lib.rs#L51)
pub const EID_HVC: usize = 0x485643;

/// Configuration for creating a new `RISCVVCpu`
#[derive(Clone, Debug)]
pub struct RISCVVCpuCreateConfig {
    /// The ID of the vCPU, default to `0`.
    pub hart_id: usize,
    /// The physical address of the device tree blob.
    /// Default to `0x9000_0000`.
    pub dtb_addr: axaddrspace::GuestPhysAddr,
}

impl Default for RISCVVCpuCreateConfig {
    fn default() -> Self {
        Self {
            hart_id: 0,
            dtb_addr: axaddrspace::GuestPhysAddr::from_usize(0x9000_0000),
        }
    }
}

pub fn send_ipi(to: usize) {
    sbi_rt::send_ipi(sbi_rt::HartMask::from_mask_base(1 << to, 0));
}
