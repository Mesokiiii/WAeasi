//! Intel e1000 (82540EM) — legacy bare-metal NIC.  Useful for QEMU
//! `-net nic,model=e1000`.  Stage 1 = stub.
pub fn probe() {
    log::debug!("[e1000] probe: not implemented in stage 1");
}

pub fn send_frame(_frame: &[u8]) -> Result<(), &'static str> {
    Err("e1000 not implemented")
}
