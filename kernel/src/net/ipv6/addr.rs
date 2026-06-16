//! IPv6 address utilities (RFC 4291).
//!
//! Beyond the 16-byte storage, an `Ipv6Addr` carries a **scope** that
//! determines whether it can be routed off-link.  Classification helpers
//! exposed here are constant-time over the public address bits — they
//! never touch the wire.
use core::fmt;

#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Ipv6Addr(pub [u8; 16]);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Scope {
    Unspecified,        // ::
    Loopback,           // ::1
    LinkLocal,          // fe80::/10
    UniqueLocal,        // fc00::/7
    Multicast,          // ff00::/8
    Documentation,      // 2001:db8::/32
    GlobalUnicast,      // 2000::/3 (everything else)
}

impl Ipv6Addr {
    pub const UNSPECIFIED: Self = Self([0; 16]);
    pub const LOOPBACK:    Self = Self([0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1]);

    /// All-nodes link-local multicast — `ff02::1`.
    pub const ALL_NODES:   Self = Self([0xFF,0x02,0,0,0,0,0,0,0,0,0,0,0,0,0,1]);
    /// All-routers link-local multicast — `ff02::2`.
    pub const ALL_ROUTERS: Self = Self([0xFF,0x02,0,0,0,0,0,0,0,0,0,0,0,0,0,2]);

    pub fn is_unspecified(&self) -> bool { self.0.iter().all(|&b| b == 0) }
    pub fn is_loopback(&self)    -> bool { *self == Self::LOOPBACK }
    pub fn is_multicast(&self)   -> bool { self.0[0] == 0xFF }
    pub fn is_link_local(&self)  -> bool { self.0[0] == 0xFE && (self.0[1] & 0xC0) == 0x80 }
    pub fn is_unique_local(&self)-> bool { (self.0[0] & 0xFE) == 0xFC }
    pub fn is_documentation(&self) -> bool {
        self.0[0]==0x20 && self.0[1]==0x01 && self.0[2]==0x0D && self.0[3]==0xB8
    }

    pub fn scope(&self) -> Scope {
        if self.is_unspecified()  { return Scope::Unspecified; }
        if self.is_loopback()     { return Scope::Loopback; }
        if self.is_link_local()   { return Scope::LinkLocal; }
        if self.is_unique_local() { return Scope::UniqueLocal; }
        if self.is_multicast()    { return Scope::Multicast; }
        if self.is_documentation(){ return Scope::Documentation; }
        Scope::GlobalUnicast
    }

    /// `true` if this address can be sent to a remote network without
    /// further translation — i.e. it's globally routable.
    #[inline]
    pub fn is_globally_routable(&self) -> bool {
        matches!(self.scope(), Scope::GlobalUnicast | Scope::UniqueLocal)
    }

    /// Solicited-node multicast address: `ff02::1:ffXX:XXXX` where the
    /// last 24 bits come from the unicast address.  Used by NDP.
    pub fn solicited_node(&self) -> Self {
        let mut a = [0u8; 16];
        a[0] = 0xFF; a[1] = 0x02;
        a[11] = 0x01; a[12] = 0xFF;
        a[13] = self.0[13]; a[14] = self.0[14]; a[15] = self.0[15];
        Self(a)
    }

    /// Build a link-local address from a 64-bit interface identifier.
    pub fn link_local_from_iid(iid: u64) -> Self {
        let mut a = [0u8; 16];
        a[0] = 0xFE; a[1] = 0x80;
        a[8..16].copy_from_slice(&iid.to_be_bytes());
        Self(a)
    }

    pub fn segments(&self) -> [u16; 8] {
        let mut s = [0u16; 8];
        for i in 0..8 { s[i] = u16::from_be_bytes([self.0[i*2], self.0[i*2+1]]); }
        s
    }
}

impl fmt::Debug for Ipv6Addr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = self.segments();
        // Find the longest run of zeros for `::` compression (RFC 5952).
        let mut best_start = 0; let mut best_len = 0;
        let mut cur_start  = 0; let mut cur_len  = 0;
        for i in 0..8 {
            if s[i] == 0 { if cur_len == 0 { cur_start = i; } cur_len += 1; }
            else { if cur_len > best_len { best_start = cur_start; best_len = cur_len; } cur_len = 0; }
        }
        if cur_len > best_len { best_start = cur_start; best_len = cur_len; }
        if best_len < 2 { best_len = 0; }

        let mut first = true;
        let mut i = 0;
        while i < 8 {
            if i == best_start && best_len > 0 {
                f.write_str("::")?;
                i += best_len;
                first = true;
                continue;
            }
            if !first { f.write_str(":")?; }
            write!(f, "{:x}", s[i])?;
            first = false;
            i += 1;
        }
        Ok(())
    }
}
