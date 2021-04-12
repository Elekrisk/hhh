use core::ops::Add;

use x86_64::structures::idt::{InterruptStackFrame, PageFaultErrorCode};

use crate::writer;


pub extern "x86-interrupt" fn alignment_check(_stack_frame: InterruptStackFrame, _error_code: u64) {
    writer::write_str("\n");
    loop {}
}

pub extern "x86-interrupt" fn bound_range_exceeded(_stack_frame: InterruptStackFrame) {
    writer::write_str("\nbound_range_exceeded");
    loop {}
}

pub extern "x86-interrupt" fn debug(_stack_frame: InterruptStackFrame) {
    writer::write_str("\ndebug");
    loop {}
}

pub extern "x86-interrupt" fn device_not_available(_stack_frame: InterruptStackFrame) {
    writer::write_str("\ndevice_not_available");
    loop {}
}

pub extern "x86-interrupt" fn divide_error(_stack_frame: InterruptStackFrame) {
    writer::write_str("\ndivide_error");
    loop {}
}

pub extern "x86-interrupt" fn general_protection_fault(stack_frame: InterruptStackFrame, error_code: u64) {
    writer::write_str("\ngeneral_protection_fault\n");
    writer::write_str("ss: ");
    writer::write_hex(error_code);
    writer::write_str("\nip: ");
    let ip = stack_frame.instruction_pointer;
    writer::write_hex(ip.as_u64());
    writer::write_str("\n");
    let code = ip.as_ptr::<u8>();
    let mut bytes = [0; 32];
    for i in 0..bytes.len() {
        bytes[i] = unsafe { code.add(i).read_volatile() };
    }
    writer::write_bytes_hex(&bytes);
    loop {}
}

pub extern "x86-interrupt" fn invalid_opcode(_stack_frame: InterruptStackFrame) {
    writer::write_str("\ninvalid_opcode");
    loop {}
}

pub extern "x86-interrupt" fn invalid_tss(_stack_frame: InterruptStackFrame, _error_code: u64) {
    writer::write_str("\ninvalid_tss");
    loop {}
}

pub extern "x86-interrupt" fn machine_check(_stack_frame: InterruptStackFrame) -> ! {
    writer::write_str("\nmachine_check");
    loop {}
}

pub extern "x86-interrupt" fn non_maskable_interrupt(_stack_frame: InterruptStackFrame) {
    writer::write_str("\nnon_maskable_interrupt");
    loop {}
}

pub extern "x86-interrupt" fn overflow(_stack_frame: InterruptStackFrame) {
    writer::write_str("\noverflow");
    loop {}
}

pub extern "x86-interrupt" fn security_exception(_stack_frame: InterruptStackFrame, _error_code: u64) {
    writer::write_str("\nsecurity_exception");
    loop {}
}

pub extern "x86-interrupt" fn segment_not_present(_stack_frame: InterruptStackFrame, _error_code: u64) {
    writer::write_str("\nsegment_not_present");
    loop {}
}

pub extern "x86-interrupt" fn simd_floating_point(_stack_frame: InterruptStackFrame) {
    writer::write_str("\nsimd_floating_point");
    loop {}
}

pub extern "x86-interrupt" fn stack_segment_fault(_stack_frame: InterruptStackFrame, _error_code: u64) {
    writer::write_str("\nstack_segment_fault");
    loop {}
}

pub extern "x86-interrupt" fn virtualization(_stack_frame: InterruptStackFrame) {
    writer::write_str("\nvirtualization");
    loop {}
}

pub extern "x86-interrupt" fn x87_floating_point(_stack_frame: InterruptStackFrame) {
    writer::write_str("\nx87_floating_point");
    loop {}
}

pub extern "x86-interrupt" fn breakpoint(_stack_frame: InterruptStackFrame) {
    writer::write_str("\nbreakpoint");
    loop {}
}

pub extern "x86-interrupt" fn page_fault(_stack_frame: InterruptStackFrame, _error_code: PageFaultErrorCode) {
    writer::write_str("\npage fault");
    loop {}
}

pub extern "x86-interrupt" fn double_fault(_stack_frame: InterruptStackFrame, _error_code: u64) -> ! {
    writer::write_str("\ndouble fault");
    loop {}
}