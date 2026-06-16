//! Common virtio plumbing — sub-drivers (`virtio_net`, `virtio_blk`)
//! ride on top.
//!
//! Layered:
//!   * `descriptor`   — the on-RAM descriptor structs.
//!   * `ring`         — available + used rings.
//!   * `queue`        — high-level VirtQueue (RAII wrapper).
//!   * `features`     — feature negotiation bits.
//!   * `pci_transport` — VirtIO over PCI (legacy + modern config).
pub mod descriptor;
pub mod features;
pub mod pci_transport;
pub mod queue;
pub mod ring;

pub use queue::VirtQueue;
