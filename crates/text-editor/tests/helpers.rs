#![allow(dead_code)]

use crop::RopeSlice;

/// Combines several rope slices into one contiguous string
/// for checking test output.
pub fn join_rope_slice<'a>(vec: &Vec<RopeSlice<'a>>) -> String {
    let mut s = String::new();
    vec.iter()
        .for_each(|chunk| s.push_str(&format!("{}\n", chunk)));
    s
}
