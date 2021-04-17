use crate::exceptions::*;
use spin::Mutex;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

static IDT: Mutex<InterruptDescriptorTable> = Mutex::new(InterruptDescriptorTable::new());

pub unsafe fn initialize_idt() {
    let mut idt = IDT.lock();
    idt.alignment_check.set_handler_fn(alignment_check);
    idt.bound_range_exceeded
        .set_handler_fn(bound_range_exceeded);
    idt.breakpoint.set_handler_fn(breakpoint);
    idt.debug.set_handler_fn(debug);
    idt.device_not_available
        .set_handler_fn(device_not_available);
    idt.divide_error.set_handler_fn(divide_error);
    idt.double_fault.set_handler_fn(double_fault);
    idt.general_protection_fault
        .set_handler_fn(general_protection_fault);
    idt.invalid_opcode.set_handler_fn(invalid_opcode);
    idt.invalid_tss.set_handler_fn(invalid_tss);
    idt.machine_check.set_handler_fn(machine_check);
    idt.non_maskable_interrupt
        .set_handler_fn(non_maskable_interrupt);
    idt.overflow.set_handler_fn(overflow);
    idt.page_fault.set_handler_fn(page_fault);
    idt.security_exception.set_handler_fn(security_exception);
    idt.segment_not_present.set_handler_fn(segment_not_present);
    idt.simd_floating_point.set_handler_fn(simd_floating_point);
    idt.stack_segment_fault.set_handler_fn(stack_segment_fault);
    idt.virtualization.set_handler_fn(virtualization);
    idt.x87_floating_point.set_handler_fn(x87_floating_point);
    // Safe, as `IDT` is static and `idt` is a reference to it
    // While `idt` may not be a static reference to allow other references (both mutable and not)
    // to it at different times, in this case, the data the reference points to will never change position
    // and will stay for the whole lifetime of the program, so in this case, the reference can be thought of
    // as "static".
    idt.load_unsafe();
}

/// Note that IRQ's start at index 32 (0x20)
pub unsafe fn register_isr(index: usize, handler: extern "x86-interrupt" fn(InterruptStackFrame)) {
    let mut idt = IDT.lock();
    idt[index].set_handler_fn(handler);
}
