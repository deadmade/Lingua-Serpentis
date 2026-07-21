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
#[derive(Debug, Default)]
pub struct Tables {
    pub stems: HashMap<String, Vec<Group>>,
    pub endings: HashMap<Group, HashSet<String>>,
    pub whole: HashSet<String>,
}

impl Tables {
    pub fn new() -> Self {
        Self::default()
    }
}
