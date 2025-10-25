//! Lines of Code (LOC) Metrics
//!
//! This module implements various line counting metrics:
//! - SLOC: Source Lines of Code (physical lines)
//! - PLOC: Physical Lines of Code (lines with actual code)
//! - CLOC: Comment Lines of Code
//! - LLOC: Logical Lines of Code
//! - BLANK: Blank lines

use serde::{Serialize, Deserialize};
use std::collections::HashSet;
use std::fmt;

/// Complete LOC metrics statistics
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LocStats {
    /// Source lines of code (total lines)
    sloc: Sloc,
    /// Physical lines of code (lines with instructions)
    ploc: Ploc,
    /// Comment lines of code
    cloc: Cloc,
    /// Logical lines of code
    lloc: Lloc,
    /// Blank lines
    blank: Blank,
}

impl Default for LocStats {
    fn default() -> Self {
        Self {
            sloc: Sloc::default(),
            ploc: Ploc::default(),
            cloc: Cloc::default(),
            lloc: Lloc::default(),
            blank: Blank::default(),
        }
    }
}

impl LocStats {
    /// Creates a new LocStats instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns SLOC value
    pub fn sloc(&self) -> f64 {
        self.sloc.sloc()
    }

    /// Returns PLOC value
    pub fn ploc(&self) -> f64 {
        self.ploc.ploc()
    }

    /// Returns CLOC value
    pub fn cloc(&self) -> f64 {
        self.cloc.cloc()
    }

    /// Returns LLOC value
    pub fn lloc(&self) -> f64 {
        self.lloc.lloc()
    }

    /// Returns blank lines count
    pub fn blank(&self) -> f64 {
        self.blank.blank()
    }

    /// Merges another LocStats into this one
    pub fn merge(&mut self, other: &LocStats) {
        self.sloc.merge(&other.sloc);
        self.ploc.merge(&other.ploc);
        self.cloc.merge(&other.cloc);
        self.lloc.merge(&other.lloc);
        self.blank.merge(&other.blank);
    }

    /// Computes min/max values
    pub fn compute_minmax(&mut self) {
        self.sloc.compute_minmax();
        self.ploc.compute_minmax();
        self.cloc.compute_minmax();
        self.lloc.compute_minmax();
        self.blank.compute_minmax();
    }
}

impl fmt::Display for LocStats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "sloc: {}, ploc: {}, cloc: {}, lloc: {}, blank: {}",
            self.sloc(),
            self.ploc(),
            self.cloc(),
            self.lloc(),
            self.blank()
        )
    }
}

/// SLOC (Source Lines of Code) metric
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Sloc {
    start: usize,
    end: usize,
    unit: bool,
    sloc_min: usize,
    sloc_max: usize,
}

impl Default for Sloc {
    fn default() -> Self {
        Self {
            start: 0,
            end: 0,
            unit: false,
            sloc_min: usize::MAX,
            sloc_max: 0,
        }
    }
}

impl Sloc {
    pub fn new(start: usize, end: usize, unit: bool) -> Self {
        Self {
            start,
            end,
            unit,
            sloc_min: usize::MAX,
            sloc_max: 0,
        }
    }

    pub fn sloc(&self) -> f64 {
        let sloc = if self.unit {
            self.end - self.start
        } else {
            (self.end - self.start) + 1
        };
        sloc as f64
    }

    pub fn sloc_min(&self) -> f64 {
        if self.sloc_min == usize::MAX {
            0.0
        } else {
            self.sloc_min as f64
        }
    }

    pub fn sloc_max(&self) -> f64 {
        self.sloc_max as f64
    }

    pub fn merge(&mut self, other: &Sloc) {
        self.sloc_min = self.sloc_min.min(other.sloc() as usize);
        self.sloc_max = self.sloc_max.max(other.sloc() as usize);
    }

    pub fn compute_minmax(&mut self) {
        if self.sloc_min == usize::MAX {
            self.sloc_min = self.sloc_min.min(self.sloc() as usize);
            self.sloc_max = self.sloc_max.max(self.sloc() as usize);
        }
    }
}

/// PLOC (Physical Lines of Code) metric
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Ploc {
    lines: HashSet<usize>,
    ploc_min: usize,
    ploc_max: usize,
}

impl Default for Ploc {
    fn default() -> Self {
        Self {
            lines: HashSet::new(),
            ploc_min: usize::MAX,
            ploc_max: 0,
        }
    }
}

impl Ploc {
    pub fn ploc(&self) -> f64 {
        self.lines.len() as f64
    }

    pub fn ploc_min(&self) -> f64 {
        if self.ploc_min == usize::MAX {
            0.0
        } else {
            self.ploc_min as f64
        }
    }

    pub fn ploc_max(&self) -> f64 {
        self.ploc_max as f64
    }

    pub fn add_line(&mut self, line: usize) {
        self.lines.insert(line);
    }

    pub fn merge(&mut self, other: &Ploc) {
        for l in other.lines.iter() {
            self.lines.insert(*l);
        }
        self.ploc_min = self.ploc_min.min(other.ploc() as usize);
        self.ploc_max = self.ploc_max.max(other.ploc() as usize);
    }

    pub fn compute_minmax(&mut self) {
        if self.ploc_min == usize::MAX {
            self.ploc_min = self.ploc_min.min(self.ploc() as usize);
            self.ploc_max = self.ploc_max.max(self.ploc() as usize);
        }
    }
}

/// CLOC (Comment Lines of Code) metric
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Cloc {
    only_comment_lines: usize,
    code_comment_lines: usize,
    cloc_min: usize,
    cloc_max: usize,
}

impl Default for Cloc {
    fn default() -> Self {
        Self {
            only_comment_lines: 0,
            code_comment_lines: 0,
            cloc_min: usize::MAX,
            cloc_max: 0,
        }
    }
}

impl Cloc {
    pub fn cloc(&self) -> f64 {
        (self.only_comment_lines + self.code_comment_lines) as f64
    }

    pub fn cloc_min(&self) -> f64 {
        if self.cloc_min == usize::MAX {
            0.0
        } else {
            self.cloc_min as f64
        }
    }

    pub fn cloc_max(&self) -> f64 {
        self.cloc_max as f64
    }

    pub fn add_only_comment_line(&mut self) {
        self.only_comment_lines += 1;
    }

    pub fn add_code_comment_line(&mut self) {
        self.code_comment_lines += 1;
    }

    pub fn merge(&mut self, other: &Cloc) {
        self.only_comment_lines += other.only_comment_lines;
        self.code_comment_lines += other.code_comment_lines;
        self.cloc_min = self.cloc_min.min(other.cloc() as usize);
        self.cloc_max = self.cloc_max.max(other.cloc() as usize);
    }

    pub fn compute_minmax(&mut self) {
        if self.cloc_min == usize::MAX {
            self.cloc_min = self.cloc_min.min(self.cloc() as usize);
            self.cloc_max = self.cloc_max.max(self.cloc() as usize);
        }
    }
}

/// LLOC (Logical Lines of Code) metric
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Lloc {
    logical_lines: usize,
    lloc_min: usize,
    lloc_max: usize,
}

impl Default for Lloc {
    fn default() -> Self {
        Self {
            logical_lines: 0,
            lloc_min: usize::MAX,
            lloc_max: 0,
        }
    }
}

impl Lloc {
    pub fn lloc(&self) -> f64 {
        self.logical_lines as f64
    }

    pub fn lloc_min(&self) -> f64 {
        if self.lloc_min == usize::MAX {
            0.0
        } else {
            self.lloc_min as f64
        }
    }

    pub fn lloc_max(&self) -> f64 {
        self.lloc_max as f64
    }

    pub fn increment(&mut self) {
        self.logical_lines += 1;
    }

    pub fn merge(&mut self, other: &Lloc) {
        self.logical_lines += other.logical_lines;
        self.lloc_min = self.lloc_min.min(other.lloc() as usize);
        self.lloc_max = self.lloc_max.max(other.lloc() as usize);
    }

    pub fn compute_minmax(&mut self) {
        if self.lloc_min == usize::MAX {
            self.lloc_min = self.lloc_min.min(self.lloc() as usize);
            self.lloc_max = self.lloc_max.max(self.lloc() as usize);
        }
    }
}

/// Blank lines metric
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Blank {
    blank_lines: usize,
    blank_min: usize,
    blank_max: usize,
}

impl Default for Blank {
    fn default() -> Self {
        Self {
            blank_lines: 0,
            blank_min: usize::MAX,
            blank_max: 0,
        }
    }
}

impl Blank {
    pub fn blank(&self) -> f64 {
        self.blank_lines as f64
    }

    pub fn blank_min(&self) -> f64 {
        if self.blank_min == usize::MAX {
            0.0
        } else {
            self.blank_min as f64
        }
    }

    pub fn blank_max(&self) -> f64 {
        self.blank_max as f64
    }

    pub fn increment(&mut self) {
        self.blank_lines += 1;
    }

    pub fn merge(&mut self, other: &Blank) {
        self.blank_lines += other.blank_lines;
        self.blank_min = self.blank_min.min(other.blank() as usize);
        self.blank_max = self.blank_max.max(other.blank() as usize);
    }

    pub fn compute_minmax(&mut self) {
        if self.blank_min == usize::MAX {
            self.blank_min = self.blank_min.min(self.blank() as usize);
            self.blank_max = self.blank_max.max(self.blank() as usize);
        }
    }
}

/// Computes LOC metrics for source code
pub fn compute_loc_metrics(source: &str) -> LocStats {
    let mut stats = LocStats::new();
    let lines: Vec<&str> = source.lines().collect();
    let total_lines = lines.len();

    stats.sloc = Sloc::new(0, total_lines, false);

    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            stats.blank.increment();
        } else {
            stats.ploc.add_line(idx);
            stats.lloc.increment();

            // Simple comment detection (can be improved)
            if trimmed.starts_with("//") || trimmed.starts_with("#") {
                stats.cloc.add_only_comment_line();
            } else if trimmed.contains("//") || trimmed.contains("/*") {
                stats.cloc.add_code_comment_line();
            }
        }
    }

    stats.compute_minmax();
    stats
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loc_metrics_simple() {
        let source = r#"
fn main() {
    println!("Hello");
}
"#;
        let stats = compute_loc_metrics(source);
        assert!(stats.sloc() > 0.0);
        assert!(stats.ploc() > 0.0);
    }

    #[test]
    fn test_blank_lines() {
        let source = r#"
fn main() {

    println!("Hello");

}
"#;
        let stats = compute_loc_metrics(source);
        assert!(stats.blank() > 0.0);
    }

    #[test]
    fn test_comments() {
        let source = r#"
// This is a comment
fn main() {
    println!("Hello"); // inline comment
}
"#;
        let stats = compute_loc_metrics(source);
        assert!(stats.cloc() > 0.0);
    }
}
