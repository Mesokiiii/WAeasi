//! Manifest schema types.

#[derive(Debug, Clone, Default)]
pub struct Manifest {
    pub name:         String,
    pub version:      String,
    pub digest:       Option<String>,
    pub capabilities: Capabilities,
    pub resources:    Resources,
    pub exports:      Vec<(String, String)>,
}

#[derive(Debug, Clone, Default)]
pub struct Capabilities { pub rights: Vec<String> }

#[derive(Debug, Clone, Default)]
pub struct Resources {
    pub cpu_shares:        Option<u32>,
    pub memory_pages_max:  Option<u32>,
    pub linear_mem_max:    Option<String>,   // human-readable; parsed downstream
}
