use std::vec;

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

pub struct Table {
    values: Vec<i32>,
    seeds: Vec<i32>,
}

struct Entry {
    idx: i32,
    hash: u64,
}

fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

impl Table {
    pub fn new(keys: &[&str]) -> Table {
        let size = (keys.len()).next_power_of_two();
        let mut h: Vec<Vec<Entry>> = Vec::with_capacity(size);
        for _ in 0..size {
            h.push(Vec::new())
        }

        for (idx, k) in keys.iter().enumerate() {
            let hash = calculate_hash(k);
            let i = hash % (size as u64);
            // idx+1 so we can identify empty entries in the table with 0
            h[i as usize].push(Entry {
                idx: (idx + 1) as i32,
                hash,
            });
        }

        h.sort_by(|a, b| b.len().cmp(&a.len()));

        let mut values = vec![0i32; size];
        let mut seeds = vec![0i32; size];

        let mut hidx = 0;

        for idx in 0..h.len() {
            hidx = idx;
            if h[hidx].len() <= 1 {
                break;
            }

            let subkeys = &h[hidx];

            let mut seed = 0u64;
            let mut entries: HashMap<usize, i32> = HashMap::new();

            'newseed: loop {
                seed += 1;
                for k in subkeys.iter() {
                    let i = (xorshift_mult64(k.hash + seed) as usize) % size;
                    if entries.get(&i) == None && values[i] == 0 {
                        // looks free, claim it
                        entries.insert(i, k.idx);
                        continue;
                    }

                    // found a collision, reset and try a new seed
                    entries.clear();
                    continue 'newseed;
                }

                // made it through; everything got placed
                break;
            }

            // mark subkey spaces as claimed
            for (&k, &v) in entries.iter() {
                values[k] = v
            }

            // and assign this seed value for every subkey
            for k in subkeys {
                let i = (k.hash as usize) % size;
                seeds[i] = seed as i32;
            }
        }

        // find the unassigned entries in the table
        let mut free: Vec<usize> = Vec::new();
        for (i, v) in values.iter_mut().enumerate() {
            if *v == 0 {
                free.push(i);
            } else {
                // decrement idx as this is now the final value for the table
                *v -= 1;
            }
        }

        while hidx < h.len() && !h[hidx].is_empty() {
            let k = &h[hidx][0];
            let i = (k.hash as usize) % size;
            hidx += 1;

            // take a free slot
            let dst = free.pop().unwrap();

            // claim it; -1 because of the +1 at the start
            values[dst] = k.idx - 1;

            // store offset in seed as a negative; -1 so even slot 0 is negative
            seeds[i] = -(dst as i32 + 1);
        }

        Table { values, seeds }
    }

    // Query looks up an entry in the table and return the index.
    pub fn query(&self, k: &str) -> usize {
        let size = self.values.len();
        let hash = calculate_hash(&k.to_string()) as u64;
        let i = hash & (size as u64 - 1);
        let seed = self.seeds[i as usize];
        if seed < 0 {
            return self.values[(-seed - 1) as usize] as usize;
        }

        let i = xorshift_mult64(seed as u64 + hash) & (size as u64 - 1);
        self.values[i as usize] as usize
    }
}

fn xorshift_mult64(x: u64) -> u64 {
    let mut x = x;
    x = x ^ (x >> 12); // a
    x ^= x << 25; // b
    x ^= x >> 27; // c
    x.wrapping_mul(2_685_821_657_736_338_717 as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let keys = vec!["foo", "bar", "baz", "qux", "zot", "frob", "zork", "zeek"];

        let t = new(&keys);

        for (i, k) in keys.iter().enumerate() {
            assert_eq!(t.query(k), i);
        }
    }
}
