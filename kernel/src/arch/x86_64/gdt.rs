//! Global Descriptor Table.
//!
//! In long mode the GDT is mostly vestigial, but we still need:
//!   * a NULL descriptor,
//!   * a 64-bit code segment for the kernel,
//!   * a TSS (interrupt stacks via IST).
use core::mem::size_of;
use lazy_static::lazy_static;
use spin::Once;
use x86_64::instructions::tables::load_tss;
use x86_64::registers::segmentation::{Segment, CS, DS, ES, SS};
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector};
use x86_64::structures::tss::TaskStateSegment;
use x86_64::VirtAddr;

/// Index of the IST slot used for double faults.
pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

const STACK_SIZE: usize = 4096 * 5;

#[repr(align(16))]
struct AlignedStack([u8; STACK_SIZE]);
static mut DOUBLE_FAULT_STACK: AlignedStack = AlignedStack([0; STACK_SIZE]);

lazy_static! {
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            let stack_start = VirtAddr::from_ptr(unsafe { &DOUBLE_FAULT_STACK });
            stack_start + STACK_SIZE as u64
        };
        tss
    };
}

pub struct Selectors {
    pub kernel_cs: SegmentSelector,
    pub kernel_ds: SegmentSelector,
    pub tss:       SegmentSelector,
}

static GDT: Once<(GlobalDescriptorTable, Selectors)> = Once::new();

/// Build & load the kernel GDT, then reload segment registers.
pub fn init() {
    let (gdt, sel) = GDT.call_once(|| {
        let mut gdt = GlobalDescriptorTable::new();
        let kernel_cs = gdt.append(Descriptor::kernel_code_segment());
        let kernel_ds = gdt.append(Descriptor::kernel_data_segment());
        let tss       = gdt.append(Descriptor::tss_segment(&TSS));
        (gdt, Selectors { kernel_cs, kernel_ds, tss })
    });
    gdt.load();
    unsafe {
        CS::set_reg(sel.kernel_cs);
        DS::set_reg(sel.kernel_ds);
        ES::set_reg(sel.kernel_ds);
        SS::set_reg(sel.kernel_ds);
        load_tss(sel.tss);
    }
    log::debug!("[gdt] loaded ({} bytes)", size_of::<TaskStateSegment>());
}
