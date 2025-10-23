use std::collections::HashMap;
use std::path::Path;

use crate::dictionary::{ConnectionMatrix, DictEntry, Dictionary, Matcher};
use crate::error::RunomeError;

/// User dictionary format types
#[derive(Debug, Clone, PartialEq)]
pub enum UserDictFormat {
    /// IPADIC format with 13 fields: surface,left_id,right_id,cost,pos_major,pos_minor1,pos_minor2,pos_minor3,infl_type,infl_form,base_form,reading,phonetic
    Ipadic,
    /// Simplified format with 3 fields: surface,pos_major,reading (other fields get defaults)
    Simpledic,
}

/// User dictionary implementation
///
/// Supports loading CSV files in both IPADIC and simplified formats,
/// building FST for efficient lookup, and integrating with the tokenizer.
/// Uses the same pattern as system dictionary for handling multiple morpheme IDs.
pub struct UserDictionary {
    entries: Vec<DictEntry>,       // All user dictionary entries
    morpheme_index: Vec<Vec<u32>>, // Maps FST index IDs to morpheme ID vectors
    matcher: Matcher,              // FST matcher for surface form lookup
    connections: ConnectionMatrix,
}

impl UserDictionary {
    /// Create new UserDictionary from CSV file
    ///
    /// # Arguments
    /// * `csv_path` - Path to CSV file containing user dictionary entries
    /// * `format` - Format of the CSV file (IPADIC or Simpledic)
    /// * `connections` - Reference to system dictionary connection matrix
    ///
    /// # Returns
    /// * `Ok(UserDictionary)` - Successfully created user dictionary
    /// * `Err(RunomeError)` - Error if CSV parsing or FST building fails
    pub fn new(
        csv_path: &Path,
        format: UserDictFormat,
        connections: ConnectionMatrix,
    ) -> Result<Self, RunomeError> {
        let entries = Self::load_entries(csv_path, format)?;
        let (matcher, morpheme_index) = Self::build_fst(&entries)?;

        Ok(Self {
            entries,
            morpheme_index,
            matcher,
            connections,
        })
    }

    /// Create new UserDictionary from CSV file with specified encoding
    ///
    /// # Arguments
    /// * `csv_path` - Path to CSV file containing user dictionary entries
    /// * `format` - Format of the CSV file (IPADIC or Simpledic)
    /// * `encoding` - Character encoding of the CSV file (UTF-8, EUC-JP, Shift_JIS, etc.)
    /// * `connections` - Reference to system dictionary connection matrix
    ///
    /// # Returns
    /// * `Ok(UserDictionary)` - Successfully created user dictionary
    /// * `Err(RunomeError)` - Error if CSV parsing, encoding conversion, or FST building fails
    pub fn new_with_encoding(
        csv_path: &Path,
        format: UserDictFormat,
        encoding: &'static encoding_rs::Encoding,
        connections: ConnectionMatrix,
    ) -> Result<Self, RunomeError> {
        let entries = Self::load_entries_with_encoding(csv_path, format, encoding)?;
        let (matcher, morpheme_index) = Self::build_fst(&entries)?;

        Ok(Self {
            entries,
            morpheme_index,
            matcher,
            connections,
        })
    }

    /// Load dictionary entries from CSV file
    fn load_entries(
        csv_path: &Path,
        format: UserDictFormat,
    ) -> Result<Vec<DictEntry>, RunomeError> {
        let content =
            std::fs::read_to_string(csv_path).map_err(|e| RunomeError::UserDictError {
                reason: format!("Failed to read CSV file {:?}: {}", csv_path, e),
            })?;

        let mut entries = Vec::new();
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let entry = match format {
                UserDictFormat::Ipadic => Self::parse_ipadic_line(line, entries.len())?,
                UserDictFormat::Simpledic => Self::parse_simpledic_line(line, entries.len())?,
            };

            entries.push(entry);
        }

        if entries.is_empty() {
            return Err(RunomeError::UserDictError {
                reason: format!("No valid entries found in CSV file {:?}", csv_path),
            });
        }

        Ok(entries)
    }

    /// Load dictionary entries from CSV file with specified encoding
    fn load_entries_with_encoding(
        csv_path: &Path,
        format: UserDictFormat,
        encoding: &'static encoding_rs::Encoding,
    ) -> Result<Vec<DictEntry>, RunomeError> {
        // Read file as bytes
        let bytes = std::fs::read(csv_path).map_err(|e| RunomeError::UserDictError {
            reason: format!("Failed to read CSV file {:?}: {}", csv_path, e),
        })?;

        // Decode using specified encoding
        let (content, _encoding_used, had_errors) = encoding.decode(&bytes);
        if had_errors {
            return Err(RunomeError::UserDictError {
                reason: format!(
                    "Failed to decode CSV file {:?} using encoding {}",
                    csv_path,
                    encoding.name()
                ),
            });
        }

        let mut entries = Vec::new();
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let entry = match format {
                UserDictFormat::Ipadic => Self::parse_ipadic_line(line, entries.len())?,
                UserDictFormat::Simpledic => Self::parse_simpledic_line(line, entries.len())?,
            };

            entries.push(entry);
        }

        if entries.is_empty() {
            return Err(RunomeError::UserDictError {
                reason: format!("No valid entries found in CSV file {:?}", csv_path),
            });
        }

        Ok(entries)
    }

    /// Parse IPADIC format CSV line (13 fields)
    fn parse_ipadic_line(line: &str, morph_id: usize) -> Result<DictEntry, RunomeError> {
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != 13 {
            return Err(RunomeError::CsvParseError {
                line: morph_id + 1,
                reason: format!("Expected 13 fields, got {}", fields.len()),
            });
        }

        Ok(DictEntry {
            surface: fields[0].to_string(),
            left_id: fields[1]
                .parse::<u16>()
                .map_err(|e| RunomeError::CsvParseError {
                    line: morph_id + 1,
                    reason: format!("Failed to parse left_id: {}", e),
                })?,
            right_id: fields[2]
                .parse::<u16>()
                .map_err(|e| RunomeError::CsvParseError {
                    line: morph_id + 1,
                    reason: format!("Failed to parse right_id: {}", e),
                })?,
            cost: fields[3]
                .parse::<i16>()
                .map_err(|e| RunomeError::CsvParseError {
                    line: morph_id + 1,
                    reason: format!("Failed to parse cost: {}", e),
                })?,
            part_of_speech: format!("{},{},{},{}", fields[4], fields[5], fields[6], fields[7]),
            inflection_type: fields[8].to_string(),
            inflection_form: fields[9].to_string(),
            base_form: fields[10].to_string(),
            reading: fields[11].to_string(),
            phonetic: fields[12].to_string(),
            morph_id,
        })
    }

    /// Parse simplified format CSV line (3 fields)
    fn parse_simpledic_line(line: &str, morph_id: usize) -> Result<DictEntry, RunomeError> {
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != 3 {
            return Err(RunomeError::CsvParseError {
                line: morph_id + 1,
                reason: format!("Expected 3 fields, got {}", fields.len()),
            });
        }

        let surface = fields[0].to_string();
        let pos_major = fields[1].to_string();
        let reading = fields[2].to_string();

        Ok(DictEntry {
            surface: surface.clone(),
            left_id: 0,
            right_id: 0,
            cost: -32000,
            part_of_speech: format!("{},*,*,*", pos_major),
            inflection_type: "*".to_string(),
            inflection_form: "*".to_string(),
            base_form: surface,
            reading: reading.clone(),
            phonetic: reading,
            morph_id,
        })
    }

    /// Build FST from dictionary entries, supporting multiple morpheme IDs per surface form
    fn build_fst(entries: &[DictEntry]) -> Result<(Matcher, Vec<Vec<u32>>), RunomeError> {
        // Group entries by surface form to handle duplicates
        let mut surface_groups: HashMap<String, Vec<u32>> = HashMap::new();
        for (id, entry) in entries.iter().enumerate() {
            surface_groups
                .entry(entry.surface.clone())
                .or_default()
                .push(id as u32);
        }

        // Create separate morpheme index for storing multiple morpheme IDs
        let mut morpheme_index: Vec<Vec<u32>> = Vec::new();

        // Create surface form to index ID mappings
        let mut surface_to_index: Vec<(String, u64)> = surface_groups
            .iter()
            .map(|(surface, ids)| {
                // Store morpheme IDs in separate index, FST stores only the index ID
                let index_id = morpheme_index.len() as u64;
                morpheme_index.push(ids.clone());
                (surface.clone(), index_id)
            })
            .collect();

        // Sort by surface form (required for FST building)
        surface_to_index.sort_by(|a, b| a.0.cmp(&b.0));

        // Build FST
        let mut builder = fst::MapBuilder::memory();
        for (surface, index_id) in surface_to_index {
            builder.insert(surface.as_bytes(), index_id).map_err(|e| {
                RunomeError::FstBuildError {
                    reason: format!("Failed to insert '{}': {}", surface, e),
                }
            })?;
        }

        let fst_bytes = builder
            .into_inner()
            .map_err(|e| RunomeError::FstBuildError {
                reason: format!("Failed to build FST: {}", e),
            })?;

        let matcher = Matcher::new(fst_bytes)?;
        Ok((matcher, morpheme_index))
    }
    /// Decode FST index ID to morpheme IDs using separate morpheme index
    fn lookup_morpheme_ids(&self, index_id: u64) -> Vec<u32> {
        if let Some(morpheme_ids) = self.morpheme_index.get(index_id as usize) {
            morpheme_ids.clone()
        } else {
            Vec::new()
        }
    }
}

impl Dictionary for UserDictionary {
    fn lookup(&self, surface: &str) -> Result<Vec<&DictEntry>, RunomeError> {
        // Handle empty string case
        if surface.is_empty() {
            return Ok(Vec::new());
        }

        // 1. Use matcher to get index IDs matching the surface form
        let (matched, index_ids) = self.matcher.run(surface, true)?;

        // 2. If no matches found, return empty vector
        if !matched {
            return Ok(Vec::new());
        }

        let mut results = Vec::new();

        // 3. For each index ID, look up the morpheme IDs and resolve to entries
        for index_id in index_ids {
            let morpheme_ids = self.lookup_morpheme_ids(index_id);

            for morpheme_id in morpheme_ids {
                // Validate morpheme ID is within bounds
                if let Some(entry) = self.entries.get(morpheme_id as usize) {
                    // Entry is already a DictEntry - no conversion needed
                    results.push(entry);
                }
            }
        }

        Ok(results)
    }

    fn get_trans_cost(&self, left_id: u16, right_id: u16) -> Result<i16, RunomeError> {
        // Delegate to system dictionary connections
        self.connections
            .get(left_id, right_id)
            .ok_or(RunomeError::InvalidConnectionId { left_id, right_id })
    }
}

#[cfg(test)]
mod tests {
    include!("user_dict_tests.rs");
}
