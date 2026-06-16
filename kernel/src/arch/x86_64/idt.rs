//! Interrupt Descriptor Table — exception vectors only. Hardware IRQ
//! handlers live in `interrupts.rs`.
use lazy_static::lazy_static;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

use super::gdt::DOUBLE_FAULT_IST_INDEX;
use super::interrupts::register_hardware_irqs;

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint);
        idt.divide_error.set_handler_fn(divide_error);
        idt.invalid_opcode.set_handler_fn(invalid_opcode);
        idt.general_protection_fault.set_handler_fn(gpf);
        idt.page_fault.set_handler_fn(page_fault);
        unsafe {
            idt.double_fault
               .set_handler_fn(double_fault)
               .set_stack_index(DOUBLE_FAULT_IST_INDEX);
        }
        register_hardware_irqs(&mut idt);
        idt
    };
}

/// Load the IDT register on the current CPU.
pub fn init() {
    IDT.load();
    log::debug!("[idt] loaded ({} entries)", 256);
}

extern "x86-interrupt" fn breakpoint(stack: InterruptStackFrame) {
    log::warn!("#BP at {:?}", stack.instruction_pointer);
}

extern "x86-interrupt" fn divide_error(stack: InterruptStackFrame) {
    panic!("#DE divide-by-zero at {:?}", stack.instruction_pointer);
}

extern "x86-interrupt" fn invalid_opcode(stack: InterruptStackFrame) {
    panic!("#UD invalid opcode at {:?}", stack.instruction_pointer);
}

extern "x86-interrupt" fn gpf(stack: InterruptStackFrame, ec: u64) {
    panic!("#GP error={:#x} at {:?}", ec, stack.instruction_pointer);
}

extern "x86-interrupt" fn double_fault(stack: InterruptStackFrame, _ec: u64) -> ! {
    panic!("#DF double fault at {:?}", stack.instruction_pointer);
}

extern "x86-interrupt" fn page_fault(stack: InterruptStackFrame, ec: PageFaultErrorCode) {
    let cr2 = x86_64::registers::control::Cr2::read();
    panic!("#PF cr2={:?} ec={:?} ip={:?}", cr2, ec, stack.instruction_pointer);
}
