//! Utilities for parsing `diskutil` output on macOS.

use std::process::Command;
use anyhow::{Context, Result};

/// Set of partition‑type strings that signify “this is a Linux partition”
/// in `diskutil list` output.
///
/// * GPT disks say **Linux Filesystem**
/// * MBR disks say **Linux**
///
/// Put the most specific strings first so we do *O(n·m)* comparisons at worst
/// where *n* = lines, *m* = patterns (tiny – just 2 today).
const LINUX_TYPES: &[&str] = &["Linux Filesystem", "Linux"];

/// Return a vector of `/dev/diskXsY` identifiers that look like Linux volumes.
pub fn find_linux_partitions() -> Result<Vec<String>> {
    let output = Command::new("diskutil")
        .arg("list")
        .output()
        .context("failed to run `diskutil list`")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut result = Vec::new();

    for line in stdout.lines() {
        // Typical line format:
        //  1:                      Linux                         512.7 GB   disk6s1
        //          ^              ^TYPE column                     ^IDENTIFIER
        let columns: Vec<&str> = line.split_whitespace().collect();

        // Need at least three columns: index, TYPE, IDENTIFIER
        if columns.len() < 3 {
            continue;
        }

        let part_type = columns[2].trim(); // “Linux” or “Linux Filesystem”
        if LINUX_TYPES
            .iter()
            .any(|pat| part_type.eq_ignore_ascii_case(pat))
        {
            // IDENTIFIER is always the *last* column
            if let Some(id) = columns.last() {
                result.push(format!("/dev/{}", id));
            }
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tiny helper that injects fake diskutil output into the parser.
    fn find_in(text: &str) -> Vec<String> {
        let mut found = Vec::new();
        for line in text.lines() {
            let columns: Vec<&str> = line.split_whitespace().collect();
            if columns.len() >= 3 {
                let part_type = columns[2].trim();
                if super::LINUX_TYPES
                    .iter()
                    .any(|pat| part_type.eq_ignore_ascii_case(pat))
                {
                    if let Some(id) = columns.last() {
                        found.push(format!("/dev/{}", id));
                    }
                }
            }
        }
        found
    }

    #[test]
    fn detects_gpt_and_mbr() {
        let gpt = "  2: Linux Filesystem             1024.0 GB   disk5s2";
        let mbr = "  1: Linux                        512.7 GB    disk6s1";
        let disks = find_in(&format!("{gpt}\n{mbr}"));
        assert_eq!(disks, vec!["/dev/disk5s2", "/dev/disk6s1"]);
    }
}