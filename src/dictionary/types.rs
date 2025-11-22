use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DictEntry {
    pub surface: String,
    pub surface_len: u16, // Character length (not byte length)
    pub left_id: u16,
    pub right_id: u16,
    pub cost: i16,
    pub part_of_speech: String,
    pub inflection_type: String,
    pub inflection_form: String,
    pub base_form: String,
    pub reading: String,
    pub phonetic: String,
    pub morph_id: usize, // Dictionary entry index for tie-breaking in Viterbi
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CharCategory {
    pub invoke: bool,
    pub group: bool,
    pub length: u8,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CodePointRange {
    pub from: char,
    pub to: char,
    pub category: String,
    pub compat_categories: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CharDefinitions {
    pub categories: std::collections::HashMap<String, CharCategory>,
    pub code_ranges: Vec<CodePointRange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnknownEntry {
    pub left_id: u16,
    pub right_id: u16,
    pub cost: i16,
    pub part_of_speech: String,
}

pub type ConnectionMatrixRows = Vec<Vec<i16>>;
pub type UnknownEntries = std::collections::HashMap<String, Vec<UnknownEntry>>;
