use std::collections::HashMap;
use std::hash::Hash;

use hash32::Hasher;

#[derive(Hash)]
pub struct PeakPair {
    freq_a: usize,  // multiple of 1/time_window
    freq_b: usize,  // multiple of 1/time_window
    delta_t: usize, // multiple of time_window
}

#[derive(Debug, Copy, Clone)]
pub struct PairRecord {
    pub hash: u32,   // hash is u32 and not u64 since u32 implements rusqlite::Value::from
    pub time_a: u32, // multiple of 1/time_window
}

pub fn pair_from_locations(loc_a: (usize, usize), loc_b: (usize, usize)) -> PeakPair {
    PeakPair {
        freq_a: loc_a.1,
        freq_b: loc_b.1,
        delta_t: loc_b.0 - loc_a.0,
    }
}

pub fn calculate_hash<T: Hash>(t: &T) -> u32 {
    let mut s = hash32::FnvHasher::default();
    // let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish32()
}

pub fn fingerprint(
    peak_locations: &[(usize, usize)],
    window_length: f32,
    target_zone_delay_sec: f32,
    target_zone_height_hz: f32,
    target_zone_width_sec: f32,
) -> HashMap<u32, PairRecord> {
    let frequency_resolution = 1. / window_length;

    // Find pairs of peaks where one peak is in the target zone of the other
    let mut close_pairs = vec![];
    for (i, loc_a) in peak_locations.iter().enumerate() {
        for loc_b in peak_locations[i + 1..].iter() {
            // dbg!(loc_b.0, loc_a.0, target_zone_delay_sec / window_length, target_zone_width_sec / window_length);
            if loc_b.0 - loc_a.0
                > (target_zone_delay_sec / window_length + target_zone_width_sec / window_length)
                    as usize
            {
                break; // past the end of the target zone
            } else {
                // within x axis range
                if loc_b.1.abs_diff(loc_a.1)
                    < ((target_zone_height_hz / frequency_resolution) / 2.) as usize
                {
                    // within frequency range
                    // dbg!("added");
                    close_pairs.push((loc_a, loc_b))
                } else {
                    // dbg!("didn't add");
                }
            }
        }
    }

    println!("Calculating {} hashes", close_pairs.len());
    // Calculate hashes to create pair records
    let mut records = HashMap::new();
    for (&loc_a, &loc_b) in close_pairs {
        let pair = pair_from_locations(loc_a, loc_b);
        let hash = calculate_hash(&pair);
        let record = PairRecord {
            hash,
            time_a: loc_a.0 as u32,
        };
        records.insert(hash, record);
    }
    records
}
