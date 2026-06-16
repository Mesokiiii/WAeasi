//! Pattern-matching assertion tracker.
//!
//! Each `expect.pattern` from the scenario is tracked independently.
//! A line "satisfies" a pattern if it contains the pattern as a
//! substring.  `satisfied()` is true once every pattern has been seen
//! at least once.

pub struct Tracker {
    patterns: Vec<String>,
    seen:     Vec<bool>,
}

impl Tracker {
    pub fn new(patterns: Vec<String>) -> Self {
        let n = patterns.len();
        Self { patterns, seen: vec![false; n] }
    }

    pub fn observe(&mut self, line: &str) {
        for (i, pat) in self.patterns.iter().enumerate() {
            if !self.seen[i] && line.contains(pat.as_str()) {
                self.seen[i] = true;
            }
        }
    }

    pub fn satisfied(&self) -> bool { self.seen.iter().all(|&b| b) }

    pub fn unsatisfied_summary(&self) -> String {
        let missed: Vec<&String> = self.patterns.iter().enumerate()
            .filter(|(i, _)| !self.seen[*i])
            .map(|(_, p)| p)
            .collect();
        if missed.is_empty() {
            String::from("timeout (all patterns matched but child still running?)")
        } else {
            format!("missing patterns: {:?}", missed)
        }
    }
}
