use riscv::register::vsatp::Vsatp;

core::arch::global_asm!(include_str!("mem_extable.S"));

unsafe extern "C"{
    pub(crate) fn _copy_from_guest(dst: *mut u8, guest_paddr: *const u8, len: usize) -> usize;
    pub(crate) fn _copy_to_guest(dst: *mut u8, src: *const u8, len: usize) -> usize;
}

pub(crate) fn copy_form_guest(dst: &mut [u8], gpa: usize) -> usize {
    let old_vsatp = riscv::register::vsatp::read().bits();
    unsafe {
        Vsatp::from_bits(0).write();
        let ret = _copy_from_guest(dst.as_mut_ptr(), gpa as *const u8, dst.len());
        Vsatp::from_bits(old_vsatp).write();
        ret
    }
}

pub(crate) fn copy_to_guest(src: &[u8], gpa: usize) -> usize {
    let old_vsatp = riscv::register::vsatp::read().bits();
    unsafe {
        Vsatp::from_bits(0).write();
        let ret = _copy_to_guest(gpa as *mut u8, src.as_ptr(), src.len());
        Vsatp::from_bits(old_vsatp).write();
        ret
    }
}
