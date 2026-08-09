#![allow(dead_code)]

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 { 0 } else { (self.next_u64() % n as u64) as usize }
    }
}

const INTERESTING: [char; 16] =
    ['°', '"', '\'', '{', '}', '(', ')', '[', ']', ';', ',', '.', '/', ':', '\n', '5'];

pub fn mutate(src: &str, rng: &mut Rng) -> String {
    let mut cs: Vec<char> = src.chars().collect();
    if cs.is_empty() {
        return "°".to_string();
    }
    match rng.below(7) {
        0 => {
            let i = rng.below(cs.len());
            cs.remove(i);
        }
        1 => {
            let i = rng.below(cs.len());
            cs.insert(i, cs[i]);
        }
        2 => {
            let (i, j) = (rng.below(cs.len()), rng.below(cs.len()));
            cs.swap(i, j);
        }
        3 => {
            let i = rng.below(cs.len());
            cs[i] = INTERESTING[rng.below(INTERESTING.len())];
        }
        4 => {
            let i = rng.below(cs.len());
            cs.truncate(i);
        }
        5 => {
            let i = rng.below(cs.len());
            cs.insert(i, INTERESTING[rng.below(INTERESTING.len())]);
        }
        _ => {
            let i = rng.below(cs.len());
            let len = 1 + rng.below(16.min(cs.len() - i));
            let span: Vec<char> = cs[i..i + len].to_vec();
            let at = rng.below(cs.len());
            for (k, c) in span.into_iter().enumerate() {
                cs.insert(at + k, c);
            }
        }
    }
    cs.into_iter().collect()
}

pub fn seed_corpus(repo_root: &std::path::Path) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for base in ["examples", "conformance"] {
        collect(&repo_root.join(base), &mut out);
    }
    out.sort();
    out
}

fn collect(dir: &std::path::Path, out: &mut Vec<(String, String)>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect(&p, out);
        } else if p.extension().is_some_and(|x| x == "liss") {
            if let Ok(s) = std::fs::read_to_string(&p) {
                out.push((p.display().to_string(), s));
            }
        }
    }
}
