use std::collections::{HashMap, HashSet};

/// The parts of speech whose forms we build from `stem + ending`.
/// Everything else (adverbs, prepositions, conjunctions, pronouns, numerals,
/// interjections) is treated as an indeclinable whole word instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pos {
    Noun,
    Adj,
    Verb,
}

/// The join key between a dictionary stem and an inflection ending.
/// `decl` is the declension number for nouns/adjectives, or the conjugation
/// number for verbs. Whitaker's "variant" is deliberately collapsed away — see
/// the leniency note in the design spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Group {
    pub pos: Pos,
    pub decl: u8,
}

/// The compiled dictionary: which stems exist, which endings each group allows,
/// and which words stand alone with no ending at all.
#[derive(Debug)]
pub struct Tables {
    pub stems: HashMap<String, Vec<Group>>,
    pub endings: HashMap<Group, HashSet<String>>,
    pub whole: HashSet<String>,
    /// Length of the longest key in `stems`, and of the longest string in any
    /// `endings` set. `analyze` uses them to skip stem/ending splits that
    /// cannot possibly match — see defectus D-23.
    pub max_stem: usize,
    pub max_ending: usize,
}

/// Deliberately hand-written rather than derived: the two bounds must default
/// to `usize::MAX`, not to `0`. A table that never calls `narrow_bounds` is
/// then merely as slow as it was before D-23, instead of silently rejecting
/// every word. The failure mode of forgetting to narrow is a performance
/// regression, which is recoverable; a false negative in the Latinity checker
/// is not.
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

    /// Record how long the longest stem and ending actually are. Call once,
    /// after every stem and ending has been inserted; calling it earlier
    /// under-counts and would turn the optimisation into a wrong answer.
    pub fn narrow_bounds(&mut self) {
        self.max_stem = self.stems.keys().map(|s| s.len()).max().unwrap_or(0);
        self.max_ending =
            self.endings.values().flatten().map(|e| e.len()).max().unwrap_or(0);
    }
}
