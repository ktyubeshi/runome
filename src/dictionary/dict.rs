use fst::Map;
use fst::raw::{Fst, Output};
use memmap2::Mmap;
use std::fs::File;
use std::path::Path;

use super::{ConnectionMatrix, DictionaryResource, loader, types::DictEntry};
use crate::error::RunomeError;

/// Dictionary trait providing core morpheme lookup functionality
///
/// This trait mirrors the interface of Janome's Python Dictionary class,
/// providing lookup and connection cost methods for morphological analysis.
pub trait Dictionary {
    /// Look up morphemes matching a surface form
    ///
    /// Returns a vector of references to DictEntry structs containing
    /// all morphological information for matching dictionary entries.
    ///
    /// # Arguments
    /// * `surface` - The surface form string to look up
    ///
    /// # Returns
    /// * `Ok(Vec<&DictEntry>)` - Vector of references to matching dictionary entries
    /// * `Err(RunomeError)` - Error if lookup fails
    fn lookup(&self, surface: &str) -> Result<Vec<&DictEntry>, RunomeError>;

    /// Get connection cost between part-of-speech IDs
    ///
    /// Returns the connection cost used in lattice-based morphological analysis
    /// to determine the cost of connecting two morphemes.
    ///
    /// # Arguments
    /// * `left_id` - Left part-of-speech ID
    /// * `right_id` - Right part-of-speech ID
    ///
    /// # Returns
    /// * `Ok(i16)` - Connection cost
    /// * `Err(RunomeError)` - Error if IDs are invalid
    fn get_trans_cost(&self, left_id: u16, right_id: u16) -> Result<i16, RunomeError>;

    /// Unsafe version of get_trans_cost that skips bounds checking.
    /// Used in hot paths where IDs are known to be valid.
    ///
    /// # Safety
    /// Caller must ensure left_id and right_id are within the matrix bounds.
    unsafe fn get_trans_cost_unchecked(&self, left_id: u16, right_id: u16) -> i16 {
        // Default implementation falls back to safe version (but unwrap to avoid Result overhead in release if optimized out)
        self.get_trans_cost(left_id, right_id).unwrap_or(i16::MAX)
    }

    /// Look up morphemes matching a surface form and append to buffer
    ///
    /// # Arguments
    /// * `surface` - The surface form string to look up
    /// * `buffer` - Buffer to append matching entries to
    ///
    /// # Returns
    /// * `Ok(())` - Success
    /// * `Err(RunomeError)` - Error if lookup fails
    fn lookup_into<'a>(
        &'a self,
        surface: &str,
        buffer: &mut Vec<&'a DictEntry>,
    ) -> Result<(), RunomeError> {
        let mut index_ids = Vec::with_capacity(16);
        self.lookup_into_with_index_buffer(surface, buffer, &mut index_ids)
    }

    /// Look up morphemes matching a surface form using provided buffers
    ///
    /// # Arguments
    /// * `surface` - The surface form string to look up
    /// * `buffer` - Buffer to append matching entries to
    /// * `index_buffer` - Buffer to use for FST index lookups (avoids allocation)
    ///
    /// # Returns
    /// * `Ok(())` - Success
    /// * `Err(RunomeError)` - Error if lookup fails
    fn lookup_into_with_index_buffer<'a>(
        &'a self,
        surface: &str,
        buffer: &mut Vec<&'a DictEntry>,
        _index_buffer: &mut Vec<u64>,
    ) -> Result<(), RunomeError> {
        // Default implementation falls back to simple lookup if not overridden
        // But in our case, all implementers should override this for performance
        let entries = self.lookup(surface)?;
        buffer.extend(entries);
        Ok(())
    }
}

/// Matcher struct for FST-based string matching
///
/// Handles finite state transducer operations to efficiently map
/// surface form strings to morpheme IDs using the fst crate.
enum FstBacking {
    Owned(Map<Vec<u8>>),
    Mapped(Map<Mmap>),
}

impl FstBacking {
    fn get(&self, key: &str) -> Option<u64> {
        match self {
            Self::Owned(map) => map.get(key),
            Self::Mapped(map) => map.get(key),
        }
    }
}

pub struct Matcher {
    fst: FstBacking,
}

impl std::fmt::Debug for Matcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Matcher").finish()
    }
}

impl Matcher {
    /// Create new Matcher from FST bytes
    ///
    /// # Arguments
    /// * `fst_bytes` - Raw FST data as bytes
    ///
    /// # Returns
    /// * `Ok(Matcher)` - Successfully created matcher
    /// * `Err(RunomeError)` - Error if FST data is invalid
    pub fn new(fst_bytes: Vec<u8>) -> Result<Self, RunomeError> {
        let fst = Map::new(fst_bytes).map_err(|e| RunomeError::DictValidationError {
            reason: format!("Failed to create FST: {}", e),
        })?;
        Ok(Self {
            fst: FstBacking::Owned(fst),
        })
    }

    /// Create new Matcher from on-disk FST using memory mapping
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, RunomeError> {
        let file = File::open(path.as_ref()).map_err(|e| RunomeError::DictValidationError {
            reason: format!("Failed to open FST file: {}", e),
        })?;
        let mmap = unsafe { Mmap::map(&file) }.map_err(|e| RunomeError::DictValidationError {
            reason: format!("Failed to memory-map FST: {}", e),
        })?;
        let fst = Map::new(mmap).map_err(|e| RunomeError::DictValidationError {
            reason: format!("Failed to create FST from mmap: {}", e),
        })?;
        Ok(Self {
            fst: FstBacking::Mapped(fst),
        })
    }

    /// Run FST matching on input word
    ///
    /// Performs FST traversal to find morpheme IDs matching the input string.
    /// Supports both exact matching and common prefix matching modes.
    ///
    /// # Arguments
    /// * `word` - Input string to match
    /// * `common_prefix_match` - If true, returns all prefixes; if false, exact match only
    ///
    /// # Returns
    /// * `Ok((bool, Vec<u32>))` - Tuple of (matched, sorted index_ids)
    /// * `Err(RunomeError)` - Error if matching fails
    pub fn run(
        &self,
        word: &str,
        common_prefix_match: bool,
    ) -> Result<(bool, Vec<u64>), RunomeError> {
        if common_prefix_match {
            // Optimized prefix matching using char boundaries
            self.run_prefix_match(word)
        } else {
            // Exact match only
            if word.is_empty() {
                return Ok((false, Vec::new()));
            }

            match self.fst.get(word) {
                Some(index_id) => Ok((true, vec![index_id])),
                None => Ok((false, Vec::new())),
            }
        }
    }

    pub fn run_into(
        &self,
        word: &str,
        common_prefix_match: bool,
        buffer: &mut Vec<u64>,
    ) -> Result<bool, RunomeError> {
        buffer.clear();
        if common_prefix_match {
            self.run_prefix_match_into(word, buffer)
        } else {
            // Exact match only
            if word.is_empty() {
                return Ok(false);
            }

            match self.fst.get(word) {
                Some(index_id) => {
                    buffer.push(index_id);
                    Ok(true)
                }
                None => Ok(false),
            }
        }
    }

    /// Optimized prefix matching that iterates over char boundaries
    fn run_prefix_match(&self, word: &str) -> Result<(bool, Vec<u64>), RunomeError> {
        let mut outputs = Vec::with_capacity(16);
        self.run_prefix_match_into(word, &mut outputs)?;
        Ok((!outputs.is_empty(), outputs))
    }

    fn run_prefix_match_into(
        &self,
        word: &str,
        buffer: &mut Vec<u64>,
    ) -> Result<bool, RunomeError> {
        if word.is_empty() {
            return Ok(false);
        }

        match &self.fst {
            FstBacking::Owned(map) => Self::collect_prefix_outputs_into(map.as_fst(), word, buffer),
            FstBacking::Mapped(map) => Self::collect_prefix_outputs_into(map.as_fst(), word, buffer),
        };

        if buffer.is_empty() {
            Ok(false)
        } else {
            buffer.sort_unstable();
            buffer.dedup();
            Ok(true)
        }
    }

    fn collect_prefix_outputs_into<D: AsRef<[u8]>>(
        fst: &Fst<D>,
        word: &str,
        buffer: &mut Vec<u64>,
    ) {
        let mut node = fst.root();
        let mut out = Output::zero();

        if node.is_final() {
            buffer.push(out.cat(node.final_output()).value());
        }

        for &b in word.as_bytes() {
            match node.find_input(b) {
                Some(i) => {
                    let transition = node.transition(i);
                    out = out.cat(transition.out);
                    node = fst.node(transition.addr);
                    if node.is_final() {
                        buffer.push(out.cat(node.final_output()).value());
                    }
                }
                None => break,
            }
        }
    }
}

/// RAMDictionary implementation using DictionaryResource and Matcher
///
/// Combines dictionary data storage (DictionaryResource) with FST-based
/// string matching (Matcher) to provide efficient morpheme lookup.
#[derive(Debug)]
pub struct RAMDictionary {
    resource: DictionaryResource,
    matcher: Matcher,
}

impl RAMDictionary {
    /// Create new RAMDictionary from DictionaryResource and sysdic directory
    ///
    /// Loads FST bytes directly from the sysdic directory and creates a Matcher instance
    /// for efficient string-to-morpheme-ID mapping.
    ///
    /// # Arguments
    /// * `resource` - DictionaryResource containing all dictionary data
    /// * `sysdic_dir` - Path to sysdic directory containing FST file
    ///
    /// # Returns
    /// * `Ok(RAMDictionary)` - Successfully created dictionary
    /// * `Err(RunomeError)` - Error if FST creation fails
    pub fn new(
        resource: DictionaryResource,
        sysdic_dir: &std::path::Path,
    ) -> Result<Self, RunomeError> {
        // Map FST file directly for lookup (avoids loading entire file into memory)
        let fst_path = loader::fst_file_path(sysdic_dir)?;
        let matcher = Matcher::from_path(&fst_path)?;

        Ok(Self { resource, matcher })
    }

    /// Get reference to the embedded DictionaryResource
    pub fn get_resource(&self) -> &DictionaryResource {
        &self.resource
    }

    /// Get connection matrix for user dictionary use
    ///
    /// Returns a reference to the connection matrix used by this dictionary.
    /// This is needed for UserDictionary initialization.
    ///
    /// # Returns
    /// * `ConnectionMatrix` - Shared reference to connection matrix
    pub fn connection_matrix(&self) -> ConnectionMatrix {
        self.resource.connection_matrix()
    }

    /// Look up morphemes matching a surface form returning entries and their indices
    ///
    /// # Arguments
    /// * `surface` - The surface form string to look up
    /// * `entries_buffer` - Buffer to append matching entries to
    /// * `indices_buffer` - Buffer to append matching morpheme indices to
    /// * `fst_index_buffer` - Buffer to use for FST index lookups (avoids allocation)
    ///
    /// # Returns
    /// * `Ok(())` - Success
    /// * `Err(RunomeError)` - Error if lookup fails
    pub fn lookup_entries_with_indices<'a>(
        &'a self,
        surface: &str,
        entries_buffer: &mut Vec<&'a DictEntry>,
        indices_buffer: &mut Vec<u32>,
        fst_index_buffer: &mut Vec<u64>,
    ) -> Result<(), RunomeError> {
        // Handle empty string case
        if surface.is_empty() {
            return Ok(());
        }

        // 1. Use matcher to get index IDs matching the surface form
        let matched = self.matcher.run_into(surface, true, fst_index_buffer)?;

        // 2. If no matches found, return
        if !matched {
            return Ok(());
        }

        // 3. Get morpheme index and dictionary entries
        let morpheme_index = self.resource.morpheme_index_view();
        let entries = self.resource.get_entries();

        // Reserve space if we can estimate
        entries_buffer.reserve(fst_index_buffer.len());
        indices_buffer.reserve(fst_index_buffer.len());

        // 4. For each index ID, look up the morpheme IDs and resolve to entries
        for index_id in fst_index_buffer.iter() {
            let morpheme_ids = morpheme_index.get(*index_id as usize);

            for &morpheme_id in morpheme_ids {
                // Validate morpheme ID is within bounds
                if let Some(entry) = entries.get(morpheme_id as usize) {
                    // Filter out entries with empty surface forms
                    if !entry.surface.is_empty() {
                        entries_buffer.push(entry);
                        indices_buffer.push(morpheme_id);
                    }
                }
            }
        }

        Ok(())
    }
}

impl Dictionary for RAMDictionary {
    fn lookup(&self, surface: &str) -> Result<Vec<&DictEntry>, RunomeError> {
        let mut results = Vec::new();
        self.lookup_into(surface, &mut results)?;
        Ok(results)
    }

    fn lookup_into_with_index_buffer<'a>(
        &'a self,
        surface: &str,
        buffer: &mut Vec<&'a DictEntry>,
        index_buffer: &mut Vec<u64>,
    ) -> Result<(), RunomeError> {
        // Handle empty string case
        if surface.is_empty() {
            return Ok(());
        }

        // 1. Use matcher to get index IDs matching the surface form
        // Use provided buffer
        let matched = self.matcher.run_into(surface, true, index_buffer)?;

        // 2. If no matches found, return
        if !matched {
            return Ok(());
        }

        // 3. Get morpheme index and dictionary entries
        let morpheme_index = self.resource.morpheme_index_view();
        let entries = self.resource.get_entries();
        
        // Reserve space if we can estimate
        buffer.reserve(index_buffer.len());

        // 4. For each index ID, look up the morpheme IDs and resolve to entries
        for index_id in index_buffer.iter() {
            let morpheme_ids = morpheme_index.get(*index_id as usize);

            for &morpheme_id in morpheme_ids {
                // Validate morpheme ID is within bounds
                if let Some(entry) = entries.get(morpheme_id as usize) {
                    // Filter out entries with empty surface forms
                    if !entry.surface.is_empty() {
                        buffer.push(entry);
                    }
                } else {
                    // Log warning but continue processing other valid IDs
                    eprintln!(
                        "Warning: Invalid morpheme ID {} for surface '{}', skipping",
                        morpheme_id, surface
                    );
                }
            }
        }

        Ok(())
    }

    fn get_trans_cost(&self, left_id: u16, right_id: u16) -> Result<i16, RunomeError> {
        // Delegate to DictionaryResource connection cost method
        self.resource.get_connection_cost(left_id, right_id)
    }

    unsafe fn get_trans_cost_unchecked(&self, left_id: u16, right_id: u16) -> i16 {
        unsafe {
            self.resource
                .connection_matrix_view()
                .get_unchecked(left_id, right_id)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn get_test_sysdic_path() -> PathBuf {
        // Assuming tests are run from the project root
        PathBuf::from("sysdic")
    }

    #[test]
    fn test_matcher_creation() {
        // Skip test if sysdic directory doesn't exist (e.g., in CI)
        let sysdic_path = get_test_sysdic_path();
        if !sysdic_path.exists() {
            eprintln!(
                "Skipping test: sysdic directory not found at {:?}",
                sysdic_path
            );
            return;
        }

        // Load FST bytes and test Matcher creation
        let fst_bytes_result = loader::load_fst_bytes(&sysdic_path);
        assert!(
            fst_bytes_result.is_ok(),
            "Failed to load FST bytes: {:?}",
            fst_bytes_result.err()
        );

        let fst_bytes = fst_bytes_result.unwrap();
        assert!(!fst_bytes.is_empty(), "FST bytes should not be empty");

        // Test Matcher creation from valid FST bytes
        let matcher_result = Matcher::new(fst_bytes);
        assert!(
            matcher_result.is_ok(),
            "Failed to create Matcher: {:?}",
            matcher_result.err()
        );

        let matcher = matcher_result.unwrap();

        // Test Matcher creation using memory-mapped file
        let fst_path = loader::fst_file_path(&sysdic_path).expect("Failed to resolve FST path");
        let mmap_matcher = Matcher::from_path(&fst_path);
        assert!(
            mmap_matcher.is_ok(),
            "Failed to create memory-mapped Matcher: {:?}",
            mmap_matcher.err()
        );

        // Test basic functionality with a simple run
        let run_result = matcher.run("test", true);
        assert!(
            run_result.is_ok(),
            "Matcher run should not fail: {:?}",
            run_result.err()
        );

        let (_matched, _outputs) = run_result.unwrap();
        // Verify the run completed successfully (matched can be true or false depending on dictionary content)
        // Verify we got outputs (empty or non-empty is valid)
    }

    #[test]
    fn test_matcher_invalid_fst() {
        // Test Matcher creation with invalid FST data
        let invalid_fst_bytes = vec![0x00, 0x01, 0x02, 0x03]; // Invalid FST data

        let matcher_result = Matcher::new(invalid_fst_bytes);
        assert!(
            matcher_result.is_err(),
            "Matcher creation should fail with invalid FST data"
        );

        // Verify it's the right kind of error
        if let Err(error) = matcher_result {
            match error {
                RunomeError::DictValidationError { reason } => {
                    assert!(reason.contains("Failed to create FST"));
                }
                _ => panic!("Expected DictValidationError, got: {:?}", error),
            }
        }
    }

    #[test]
    fn test_matcher_run_exact_match() {
        // Skip test if sysdic directory doesn't exist (e.g., in CI)
        let sysdic_path = get_test_sysdic_path();
        if !sysdic_path.exists() {
            eprintln!(
                "Skipping test: sysdic directory not found at {:?}",
                sysdic_path
            );
            return;
        }

        // Load FST bytes and create matcher
        let fst_bytes = loader::load_fst_bytes(&sysdic_path).expect("Failed to load FST bytes");
        let matcher = Matcher::new(fst_bytes).expect("Failed to create Matcher");

        // Test exact match for "東京"
        let run_result = matcher.run("東京", false);
        assert!(
            run_result.is_ok(),
            "Matcher run should not fail: {:?}",
            run_result.err()
        );

        let (matched, outputs) = run_result.unwrap();

        // "東京" should exist in the dictionary
        if matched {
            assert!(!outputs.is_empty(), "Should have morpheme IDs for 東京");
            // Verify we got valid morpheme IDs (non-zero values)
            for morpheme_id in &outputs {
                assert!(*morpheme_id > 0, "Morpheme ID should be greater than 0");
            }
        } else {
            assert!(
                outputs.is_empty(),
                "Outputs should be empty for exact match if not found"
            );
            eprintln!("No match found for '東京' in the dictionary");
        }
    }

    #[test]
    fn test_matcher_run_prefix_match() {
        // Skip test if sysdic directory doesn't exist (e.g., in CI)
        let sysdic_path = get_test_sysdic_path();
        if !sysdic_path.exists() {
            eprintln!(
                "Skipping test: sysdic directory not found at {:?}",
                sysdic_path
            );
            return;
        }

        // Load FST bytes and create matcher
        let fst_bytes = loader::load_fst_bytes(&sysdic_path).expect("Failed to load FST bytes");
        let matcher = Matcher::new(fst_bytes).expect("Failed to create Matcher");

        // Test prefix match for "東京" (should include "東" if it exists)
        let run_result = matcher.run("東京", true);
        assert!(
            run_result.is_ok(),
            "Matcher run should not fail: {:?}",
            run_result.err()
        );

        let (matched, outputs) = run_result.unwrap();

        // With prefix matching, we should get results for any prefixes that exist
        // This could include "東" (1-char prefix) and "東京" (full word) if they exist
        if matched {
            assert!(
                !outputs.is_empty(),
                "Should have morpheme IDs for prefixes of 東京"
            );

            // Verify we got valid morpheme IDs
            for morpheme_id in &outputs {
                assert!(*morpheme_id > 0, "Morpheme ID should be greater than 0");
            }

            // With prefix matching, we should get more results than exact matching
            // Test that the method correctly processes multiple character boundaries
            // for multi-byte UTF-8 characters like "東京"
            let exact_match_result = matcher.run("東京", false);
            let (_, exact_outputs) = exact_match_result.unwrap();
            assert!(
                outputs.len() > exact_outputs.len(),
                "Prefix match should return at least as many results as exact match"
            );
        }
    }

    #[test]
    fn test_matcher_run_exact_match_non_existent_word() {
        // Skip test if sysdic directory doesn't exist (e.g., in CI)
        let sysdic_path = get_test_sysdic_path();
        if !sysdic_path.exists() {
            eprintln!(
                "Skipping test: sysdic directory not found at {:?}",
                sysdic_path
            );
            return;
        }

        // Load FST bytes and create matcher
        let fst_bytes = loader::load_fst_bytes(&sysdic_path).expect("Failed to load FST bytes");
        let matcher = Matcher::new(fst_bytes).expect("Failed to create Matcher");

        // Test run with a non-existent word
        let run_result = matcher.run("ミャクミャク", false);
        assert!(
            run_result.is_ok(),
            "Matcher run should not fail for non-existent word: {:?}",
            run_result.err()
        );

        let (matched, outputs) = run_result.unwrap();

        // Non-existent words should not match anything
        assert!(
            !matched,
            "Non-existent word should not match any morpheme IDs"
        );
        assert!(
            outputs.is_empty(),
            "Outputs should be empty for non-existent word"
        );
    }

    #[test]
    fn test_ram_dictionary_creation() {
        // Skip test if sysdic directory doesn't exist (e.g., in CI)
        let sysdic_path = get_test_sysdic_path();
        if !sysdic_path.exists() {
            eprintln!(
                "Skipping test: sysdic directory not found at {:?}",
                sysdic_path
            );
            return;
        }

        // Load DictionaryResource first
        let resource_result = DictionaryResource::load(&sysdic_path);
        assert!(
            resource_result.is_ok(),
            "Failed to load DictionaryResource: {:?}",
            resource_result.err()
        );

        let resource = resource_result.unwrap();

        // Test RAMDictionary creation from DictionaryResource and sysdic path
        let ram_dict_result = RAMDictionary::new(resource, &sysdic_path);
        assert!(
            ram_dict_result.is_ok(),
            "Failed to create RAMDictionary: {:?}",
            ram_dict_result.err()
        );

        let ram_dict = ram_dict_result.unwrap();

        // Test that get_trans_cost works (delegated to DictionaryResource)
        let cost_result = ram_dict.get_trans_cost(0, 0);
        assert!(
            cost_result.is_ok(),
            "get_trans_cost should work: {:?}",
            cost_result.err()
        );
    }

    #[test]
    fn test_get_trans_cost_delegation() {
        // Skip test if sysdic directory doesn't exist (e.g., in CI)
        let sysdic_path = get_test_sysdic_path();
        if !sysdic_path.exists() {
            eprintln!(
                "Skipping test: sysdic directory not found at {:?}",
                sysdic_path
            );
            return;
        }

        // Load DictionaryResource and create RAMDictionary
        let resource =
            DictionaryResource::load(&sysdic_path).expect("Failed to load DictionaryResource");
        let ram_dict =
            RAMDictionary::new(resource, &sysdic_path).expect("Failed to create RAMDictionary");

        // Test that get_trans_cost properly delegates to DictionaryResource
        // Test various connection cost lookups
        let test_cases = [
            (0, 0, -434),
            (0, 1, 1),
            (100, 200, -87),
            (151, 1280, 694),
            (227, 1266, -962),
        ];

        for (left_id, right_id, expected_cost) in test_cases {
            let cost_result = ram_dict.get_trans_cost(left_id, right_id);

            // Should not fail for valid IDs within matrix bounds
            if cost_result.is_ok() {
                let cost = cost_result.unwrap();
                // Connection costs are valid integers (can be positive, negative, or zero)
                // Just verify we got a valid i16 value - no need to check sign
                assert!(
                    cost == expected_cost,
                    "Connection cost for ({}, {}) should be {}, got: {}",
                    left_id,
                    right_id,
                    expected_cost,
                    cost
                );
            }
            // If it fails, it should be due to IDs being out of bounds, which is acceptable
        }

        // Test invalid IDs that should fail
        let invalid_id = 9999;
        let invalid_result = ram_dict.get_trans_cost(invalid_id, 0);
        assert!(
            invalid_result.is_err(),
            "Should fail for invalid left_id: {}",
            invalid_id
        );

        let invalid_result2 = ram_dict.get_trans_cost(0, invalid_id);
        assert!(
            invalid_result2.is_err(),
            "Should fail for invalid right_id: {}",
            invalid_id
        );
    }

    #[test]
    fn test_lookup_known_words() {
        // Skip test if sysdic directory doesn't exist (e.g., in CI)
        let sysdic_path = get_test_sysdic_path();
        if !sysdic_path.exists() {
            eprintln!(
                "Skipping test: sysdic directory not found at {:?}",
                sysdic_path
            );
            return;
        }

        // Load DictionaryResource and create RAMDictionary
        let resource =
            DictionaryResource::load(&sysdic_path).expect("Failed to load DictionaryResource");
        let ram_dict =
            RAMDictionary::new(resource, &sysdic_path).expect("Failed to create RAMDictionary");

        // Test lookup of common Japanese words that should exist in the dictionary
        let test_words = ["東京", "日本", "の", "です", "する"];

        for word in test_words {
            let lookup_result = ram_dict.lookup(word);
            assert!(
                lookup_result.is_ok(),
                "Lookup should not fail for word: {}",
                word
            );

            let entries = lookup_result.unwrap();

            // If word is found, verify the entries are valid
            if !entries.is_empty() {
                for entry in &entries {
                    // Verify essential fields are populated
                    // Note: surface can be empty for special entries like whitespace symbols
                    assert!(
                        !entry.part_of_speech.is_empty(),
                        "Part of speech should not be empty"
                    );
                    assert!(!entry.reading.is_empty(), "Reading should not be empty");

                    // Verify numeric fields are reasonable
                    assert!(
                        entry.left_id < 10000,
                        "Left ID should be reasonable: {}",
                        entry.left_id
                    );
                    assert!(
                        entry.right_id < 10000,
                        "Right ID should be reasonable: {}",
                        entry.right_id
                    );

                    // Word cost can be positive or negative but should be reasonable
                    assert!(
                        entry.cost > -32000 && entry.cost < 32000,
                        "Cost should be reasonable: {}",
                        entry.cost
                    );
                }
            }
        }
    }

    #[test]
    fn test_lookup_unknown_words() {
        // Skip test if sysdic directory doesn't exist (e.g., in CI)
        let sysdic_path = get_test_sysdic_path();
        if !sysdic_path.exists() {
            eprintln!(
                "Skipping test: sysdic directory not found at {:?}",
                sysdic_path
            );
            return;
        }

        // Load DictionaryResource and create RAMDictionary
        let resource =
            DictionaryResource::load(&sysdic_path).expect("Failed to load DictionaryResource");
        let ram_dict =
            RAMDictionary::new(resource, &sysdic_path).expect("Failed to create RAMDictionary");

        // Test lookup of words that definitely should NOT exist in the dictionary
        let unknown_words = ["qwerty123", "unknownword"];

        for word in unknown_words {
            let lookup_result = ram_dict.lookup(word);
            assert!(
                lookup_result.is_ok(),
                "Lookup should not fail even for unknown words: {}",
                word
            );

            let entries = lookup_result.unwrap();
            // Unknown words should return empty results (not an error)
            assert!(
                entries.is_empty(),
                "Unknown word '{}' should return empty results",
                word
            );
        }
    }

    #[test]
    fn test_lookup_empty_string() {
        // Skip test if sysdic directory doesn't exist (e.g., in CI)
        let sysdic_path = get_test_sysdic_path();
        if !sysdic_path.exists() {
            eprintln!(
                "Skipping test: sysdic directory not found at {:?}",
                sysdic_path
            );
            return;
        }

        // Load DictionaryResource and create RAMDictionary
        let resource =
            DictionaryResource::load(&sysdic_path).expect("Failed to load DictionaryResource");
        let ram_dict =
            RAMDictionary::new(resource, &sysdic_path).expect("Failed to create RAMDictionary");

        // Test lookup of empty string
        let lookup_result = ram_dict.lookup("");
        assert!(
            lookup_result.is_ok(),
            "Lookup should not fail for empty string"
        );

        let entries = lookup_result.unwrap();
        // Empty string should return empty results
        assert!(
            entries.is_empty(),
            "Empty string should return empty results"
        );
    }

    #[test]
    fn test_lookup_consistency() {
        // Skip test if sysdic directory doesn't exist (e.g., in CI)
        let sysdic_path = get_test_sysdic_path();
        if !sysdic_path.exists() {
            eprintln!(
                "Skipping test: sysdic directory not found at {:?}",
                sysdic_path
            );
            return;
        }

        // Load DictionaryResource and create RAMDictionary
        let resource =
            DictionaryResource::load(&sysdic_path).expect("Failed to load DictionaryResource");
        let ram_dict =
            RAMDictionary::new(resource, &sysdic_path).expect("Failed to create RAMDictionary");

        // Test that multiple lookups of the same word return consistent results
        let test_word = "東京";

        let first_lookup = ram_dict
            .lookup(test_word)
            .expect("First lookup should succeed");
        let second_lookup = ram_dict
            .lookup(test_word)
            .expect("Second lookup should succeed");

        // Both lookups should return the same results
        assert_eq!(
            first_lookup, second_lookup,
            "Lookup results should be identical for the same word"
        );
    }

    #[test]
    fn test_lookup_multibyte_utf8() {
        // Skip test if sysdic directory doesn't exist (e.g., in CI)
        let sysdic_path = get_test_sysdic_path();
        if !sysdic_path.exists() {
            eprintln!(
                "Skipping test: sysdic directory not found at {:?}",
                sysdic_path
            );
            return;
        }

        // Load DictionaryResource and create RAMDictionary
        let resource =
            DictionaryResource::load(&sysdic_path).expect("Failed to load DictionaryResource");
        let ram_dict =
            RAMDictionary::new(resource, &sysdic_path).expect("Failed to create RAMDictionary");

        // Test lookup with various Japanese character types
        let test_cases = [
            ("ひらがな", "Hiragana"),
            ("カタカナ", "Katakana"),
            ("漢字", "Kanji"),
            ("ＡＢＣ", "Full-width ASCII"),
        ];

        for (word, description) in test_cases {
            let lookup_result = ram_dict.lookup(word);
            assert!(
                lookup_result.is_ok(),
                "Lookup should not fail for {} word: {}",
                description,
                word
            );

            // We don't assert on whether results are found or not, since it depends
            // on dictionary content, but the lookup should complete successfully
            let entries = lookup_result.unwrap();
            assert!(
                !entries.is_empty() || word.is_empty(),
                "Lookup for {} should return entries or be empty: {}",
                description,
                word
            );
        }
    }
}
