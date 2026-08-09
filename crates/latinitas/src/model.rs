use std::collections::{HashMap, HashSet};

/// The parts of speech whose forms are built from `stem + ending`; everything
/// else is treated as an indeclinable whole word.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pos {
    Noun,
    Adj,
    Verb,
}

/// The join key between a dictionary stem and an inflection ending: `decl` is
/// the declension for nouns and adjectives, the conjugation for verbs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Group {
    pub pos: Pos,
    pub decl: u8,
}

#[derive(Debug)]
pub struct Tables {
    pub stems: HashMap<String, Vec<Group>>,
    pub endings: HashMap<Group, HashSet<String>>,
    pub whole: HashSet<String>,
    /// Longest key in `stems`, and longest string in any `endings` set. D-23
    pub max_stem: usize,
    pub max_ending: usize,
}

/// Hand-written, not derived: the bounds must default to `usize::MAX`, so that
/// a table nobody narrowed is merely slow rather than silently empty. D-23
impl Default for Tables {
    fn default() -> Self {
        Tables {
            stems: HashMap::new(),
            endings: HashMap::new(),
            whole: HashSet::new(),
            max_stem: usize::MAX,
            max_ending: usize::MAX,
        }
    }
}

impl Tables {
    pub fn new() -> Self {
        Self::default()
    }

    /// Call once, after every stem and ending has been inserted; earlier
    /// under-counts.
    pub fn narrow_bounds(&mut self) {
        self.max_stem = self.stems.keys().map(|s| s.len()).max().unwrap_or(0);
        self.max_ending =
            self.endings.values().flatten().map(|e| e.len()).max().unwrap_or(0);
    }
}
