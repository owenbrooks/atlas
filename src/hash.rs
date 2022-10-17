use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Hash)]
pub struct PeakPair {
    freq_a: usize,  // multiple of 1/time_window
    freq_b: usize,  // multiple of 1/time_window
    delta_t: usize, // multiple of time_window
}

pub fn pair_from_locations(loc_a: (usize, usize), loc_b: (usize, usize)) -> PeakPair {
    PeakPair {
        freq_a: loc_a.1,
        freq_b: loc_b.1,
        delta_t: loc_b.0 - loc_a.0,
    }
}

pub fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}
