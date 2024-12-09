use core::marker::PhantomData;

use tock_registers::LocalRegisterCopy;

use axerrno::{AxError, AxResult};
use axvcpu::{AxArchPerCpu, AxVCpuHal};

use crate::csrs::{defs::hstatus, traps, RiscvCsrTrait, CSR};
use crate::has_hardware_support;

/// Risc-V per-CPU state.
pub struct RISCVPerCpu<H: AxVCpuHal> {
    _marker: PhantomData<H>,
}

impl<H: AxVCpuHal> AxArchPerCpu for RISCVPerCpu<H> {
    fn new(_cpu_id: usize) -> AxResult<Self> {
        unsafe {
            setup_csrs();
        }

        Ok(Self {
            _marker: PhantomData,
        })
    }

    fn is_enabled(&self) -> bool {
        unimplemented!()
    }

    fn hardware_enable(&mut self) -> AxResult<()> {
        if has_hardware_support() {
            // Set hstatus
            let mut hstatus = LocalRegisterCopy::<usize, hstatus::Register>::new(
                riscv::register::hstatus::read().bits(),
            );
            hstatus.modify(hstatus::spv::Supervisor);
            // Set SPVP bit in order to accessing VS-mode memory from HS-mode.
            hstatus.modify(hstatus::spvp::Supervisor);
            CSR.hstatus.write_value(hstatus.get());

            Ok(())
        } else {
            Err(AxError::Unsupported)
        }
    }

    fn hardware_disable(&mut self) -> AxResult<()> {
        unimplemented!()
    }
}

/// Initialize (H)S-level CSRs to a reasonable state.
unsafe fn setup_csrs() {
    // Delegate some synchronous exceptions.
    CSR.hedeleg.write_value(
        traps::exception::INST_ADDR_MISALIGN
            | traps::exception::BREAKPOINT
            | traps::exception::ENV_CALL_FROM_U_OR_VU
            | traps::exception::INST_PAGE_FAULT
            | traps::exception::LOAD_PAGE_FAULT
            | traps::exception::STORE_PAGE_FAULT
            | traps::exception::ILLEGAL_INST,
    );

    // Delegate all interupts.
    CSR.hideleg.write_value(
        traps::interrupt::VIRTUAL_SUPERVISOR_TIMER
            | traps::interrupt::VIRTUAL_SUPERVISOR_EXTERNAL
            | traps::interrupt::VIRTUAL_SUPERVISOR_SOFT,
    );

    // Clear all interrupts.
    CSR.hvip.read_and_clear_bits(
        traps::interrupt::VIRTUAL_SUPERVISOR_TIMER
            | traps::interrupt::VIRTUAL_SUPERVISOR_EXTERNAL
            | traps::interrupt::VIRTUAL_SUPERVISOR_SOFT,
    );

    // clear all interrupts.
    CSR.hcounteren.write_value(0xffff_ffff);

    // enable interrupt
    CSR.sie.write_value(
        traps::interrupt::SUPERVISOR_EXTERNAL
            | traps::interrupt::SUPERVISOR_SOFT
            | traps::interrupt::SUPERVISOR_TIMER,
    );
    debug!("sie: {:#x}", CSR.sie.get_value());
}
