//! Type-safe wrappers around `in/out` instructions for legacy I/O ports.
use core::arch::asm;
use core::marker::PhantomData;

/// A typed I/O port (u8 / u16 / u32).
pub struct Port<T: PortIo> {
    addr: u16,
    _marker: PhantomData<T>,
}

impl<T: PortIo> Port<T> {
    pub const fn new(addr: u16) -> Self {
        Self { addr, _marker: PhantomData }
    }
    /// SAFETY: the caller asserts that `addr` is a real, non-conflicting
    /// I/O port and that this access does not violate device contracts.
    #[inline(always)]
    pub unsafe fn read(&self) -> T { T::port_in(self.addr) }
    #[inline(always)]
    pub unsafe fn write(&self, val: T) { T::port_out(self.addr, val) }
}

/// Marker trait describing `in`/`out` for a primitive width.
pub trait PortIo: Copy {
    unsafe fn port_in(addr: u16) -> Self;
    unsafe fn port_out(addr: u16, val: Self);
}

impl PortIo for u8 {
    #[inline(always)]
    unsafe fn port_in(addr: u16) -> u8 {
        let v: u8;
        asm!("in al, dx", in("dx") addr, out("al") v, options(nomem, nostack));
        v
    }
    #[inline(always)]
    unsafe fn port_out(addr: u16, val: u8) {
        asm!("out dx, al", in("dx") addr, in("al") val, options(nomem, nostack));
    }
}

impl PortIo for u16 {
    #[inline(always)]
    unsafe fn port_in(addr: u16) -> u16 {
        let v: u16;
        asm!("in ax, dx", in("dx") addr, out("ax") v, options(nomem, nostack));
        v
    }
    #[inline(always)]
    unsafe fn port_out(addr: u16, val: u16) {
        asm!("out dx, ax", in("dx") addr, in("ax") val, options(nomem, nostack));
    }
}

impl PortIo for u32 {
    #[inline(always)]
    unsafe fn port_in(addr: u16) -> u32 {
        let v: u32;
        asm!("in eax, dx", in("dx") addr, out("eax") v, options(nomem, nostack));
        v
    }
    #[inline(always)]
    unsafe fn port_out(addr: u16, val: u32) {
        asm!("out dx, eax", in("dx") addr, in("eax") val, options(nomem, nostack));
    }
}
