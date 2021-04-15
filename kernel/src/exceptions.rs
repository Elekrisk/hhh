use core::ops::Add;

use x86_64::structures::idt::{InterruptStackFrame, PageFaultErrorCode};


pub extern "x86-interrupt" fn alignment_check(_stack_frame: InterruptStackFrame, _error_code: u64) {
    println!("\n");
    loop {}
}

pub extern "x86-interrupt" fn bound_range_exceeded(_stack_frame: InterruptStackFrame) {
    println!("\nbound_range_exceeded");
    loop {}
}

pub extern "x86-interrupt" fn debug(_stack_frame: InterruptStackFrame) {
    println!("\ndebug");
    loop {}
}

pub extern "x86-interrupt" fn device_not_available(_stack_frame: InterruptStackFrame) {
    println!("\ndevice_not_available");
    loop {}
}

pub extern "x86-interrupt" fn divide_error(_stack_frame: InterruptStackFrame) {
    println!("\ndivide_error");
    loop {}
}

pub extern "x86-interrupt" fn general_protection_fault(stack_frame: InterruptStackFrame, error_code: u64) {
    println!("\ngeneral_protection_fault\n");
    println!("ss: {:x}", error_code);
    let ip = stack_frame.instruction_pointer;
    println!("ip: {:x}", ip);
    let code = ip.as_ptr::<u8>();
    for i in 0..32 {
        print!("{:x} ", unsafe { code.add(i).read_volatile() });
    }
    println!();
    loop {}
}

pub extern "x86-interrupt" fn invalid_opcode(_stack_frame: InterruptStackFrame) {
    println!("\ninvalid_opcode");
    loop {}
}

pub extern "x86-interrupt" fn invalid_tss(_stack_frame: InterruptStackFrame, _error_code: u64) {
    println!("\ninvalid_tss");
    loop {}
}

pub extern "x86-interrupt" fn machine_check(_stack_frame: InterruptStackFrame) -> ! {
    println!("\nmachine_check");
    loop {}
}

pub extern "x86-interrupt" fn non_maskable_interrupt(_stack_frame: InterruptStackFrame) {
    println!("\nnon_maskable_interrupt");
    loop {}
}

pub extern "x86-interrupt" fn overflow(_stack_frame: InterruptStackFrame) {
    println!("\noverflow");
    loop {}
}

pub extern "x86-interrupt" fn security_exception(_stack_frame: InterruptStackFrame, _error_code: u64) {
    println!("\nsecurity_exception");
    loop {}
}

pub extern "x86-interrupt" fn segment_not_present(_stack_frame: InterruptStackFrame, _error_code: u64) {
    println!("\nsegment_not_present");
    loop {}
}

pub extern "x86-interrupt" fn simd_floating_point(_stack_frame: InterruptStackFrame) {
    println!("\nsimd_floating_point");
    loop {}
}

pub extern "x86-interrupt" fn stack_segment_fault(_stack_frame: InterruptStackFrame, _error_code: u64) {
    println!("\nstack_segment_fault");
    loop {}
}

pub extern "x86-interrupt" fn virtualization(_stack_frame: InterruptStackFrame) {
    println!("\nvirtualization");
    loop {}
}

pub extern "x86-interrupt" fn x87_floating_point(_stack_frame: InterruptStackFrame) {
    println!("\nx87_floating_point");
    loop {}
}

pub extern "x86-interrupt" fn breakpoint(_stack_frame: InterruptStackFrame) {
    println!("\nbreakpoint");
}

pub extern "x86-interrupt" fn page_fault(stack_frame: InterruptStackFrame, _error_code: PageFaultErrorCode) {
    println!("\nPage fault while trying to access 0x{:x}", x86_64::registers::control::Cr2::read());
    println!("Caused by instruction at 0x{:x}", stack_frame.instruction_pointer);
    loop {}
}

pub extern "x86-interrupt" fn double_fault(_stack_frame: InterruptStackFrame, _error_code: u64) -> ! {
    println!("\ndouble fault");
    loop {}
}