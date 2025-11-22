use once_cell::sync::OnceCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::{CategoryMask, ConnectionMatrix, Dictionary, DictionaryResource, RAMDictionary};
use crate::dictionary::types::{DictEntry, UnknownEntry};
use crate::error::RunomeError;

/// SystemDictionary combines known word lookup with character classification
///
/// This provides the primary interface for Japanese morphological analysis,
/// integrating dictionary lookups for known words with character-based
/// classification for unknown word processing.
pub struct SystemDictionary {
    /// RAM-based dictionary for known word lookup
    ram_dict: RAMDictionary,
}

/// Singleton instance with thread-safe lazy initialization
static SYSTEM_DICT_INSTANCE: OnceCell<Arc<SystemDictionary>> = OnceCell::new();

impl SystemDictionary {
    /// Get the sysdic path, trying bundled location first, then relative path
    ///
    /// # Returns
    /// * `PathBuf` - Path to the sysdic directory
    fn get_sysdic_path() -> PathBuf {
        // Try bundled path first (set by build.rs)
        if let Ok(bundled_path) = std::env::var("SYSDIC_PATH") {
            let path = PathBuf::from(bundled_path);
            if path.exists() {
                return path;
            }
        }

        // Try Python package location (for installed packages)
        #[cfg(feature = "python")]
        {
            // First try the bundled location within the Python package
            if let Ok(module_path) = std::env::var("CARGO_MANIFEST_DIR") {
                let package_sysdic = PathBuf::from(&module_path).join("runome/sysdic");
                if package_sysdic.exists() {
                    return package_sysdic;
                }
            }

            // Try to find sysdic using Python module introspection
            // This is the most reliable way for installed packages
            // Only try this if we're running in a Python extension context
            if std::env::var("PYTHONPATH").is_ok() || std::env::var("VIRTUAL_ENV").is_ok() {
                use pyo3::prelude::*;
                if let Ok(py_result) = Python::with_gil(|py| -> PyResult<Option<PathBuf>> {
                    // Import the runome module to get its location
                    let runome_module = py.import("runome")?;
                    let file_attr = runome_module.getattr("__file__")?;
                    let module_file: String = file_attr.extract()?;

                    // Get the directory containing the module
                    let module_dir = PathBuf::from(module_file).parent().unwrap().to_path_buf();
                    let sysdic_path = module_dir.join("sysdic");

                    if sysdic_path.exists() {
                        Ok(Some(sysdic_path))
                    } else {
                        Ok(None)
                    }
                }) {
                    if let Some(path) = py_result {
                        return path;
                    }
                }
            }

            // Fallback: Try to find sysdic relative to current working directory
            let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let mut search_dir = current_dir.clone();

            // Search upward in the directory tree for a runome package
            for _ in 0..5 {
                // Limit search depth
                let candidate_path = search_dir.join("runome/sysdic");
                if candidate_path.exists() {
                    return candidate_path;
                }

                if let Some(parent) = search_dir.parent() {
                    search_dir = parent.to_path_buf();
                } else {
                    break;
                }
            }
        }

        // Try relative to current module (for Python packages)
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let relative_sysdic = current_dir.join("sysdic");
        if relative_sysdic.exists() {
            return relative_sysdic;
        }

        // Fall back to relative path for development
        PathBuf::from("sysdic")
    }

    /// Get singleton instance of SystemDictionary
    ///
    /// Returns a shared reference to the singleton SystemDictionary instance,
    /// creating it if it doesn't exist. Uses lazy initialization with thread safety.
    ///
    /// # Returns
    /// * `Ok(Arc<SystemDictionary>)` - Shared reference to singleton instance
    /// * `Err(RunomeError)` - Error if initialization fails
    pub fn instance() -> Result<Arc<SystemDictionary>, RunomeError> {
        SYSTEM_DICT_INSTANCE
            .get_or_try_init(|| {
                let sysdic_path = Self::get_sysdic_path();
                Ok(Arc::new(Self::new(&sysdic_path)?))
            })
            .map(Arc::clone)
    }

    /// Create new SystemDictionary from sysdic directory
    ///
    /// Loads dictionary data and character definitions from the specified directory.
    /// This is used internally by the singleton pattern.
    ///
    /// # Arguments  
    /// * `sysdic_dir` - Path to directory containing dictionary data
    ///
    /// # Returns
    /// * `Ok(SystemDictionary)` - Successfully created dictionary
    /// * `Err(RunomeError)` - Error if loading fails
    pub fn new(sysdic_dir: &Path) -> Result<Self, RunomeError> {
        // Load dictionary resource
        let resource = DictionaryResource::load(sysdic_dir)?;

        // Create RAMDictionary
        let ram_dict = RAMDictionary::new(resource, sysdic_dir)?;

        Ok(Self { ram_dict })
    }

    /// Look up known words only (delegates to RAMDictionary)
    ///
    /// Performs dictionary lookup for known words using the embedded RAMDictionary.
    /// This does not handle unknown word processing.
    ///
    /// # Arguments
    /// * `surface` - Surface form string to look up
    ///
    /// # Returns  
    /// * `Ok(Vec<&DictEntry>)` - Vector of references to matching dictionary entries
    /// * `Err(RunomeError)` - Error if lookup fails
    pub fn lookup(&self, surface: &str) -> Result<Vec<&DictEntry>, RunomeError> {
        self.ram_dict.lookup(surface)
    }

    /// Get connection cost between part-of-speech IDs
    ///
    /// Delegates to the embedded RAMDictionary to get connection costs
    /// used in lattice-based morphological analysis.
    ///
    /// # Arguments
    /// * `left_id` - Left part-of-speech ID
    /// * `right_id` - Right part-of-speech ID
    ///
    /// # Returns
    /// * `Ok(i16)` - Connection cost
    /// * `Err(RunomeError)` - Error if IDs are invalid
    pub fn get_trans_cost(&self, left_id: u16, right_id: u16) -> Result<i16, RunomeError> {
        self.ram_dict.get_trans_cost(left_id, right_id)
    }

    /// Get connection matrix for user dictionary use
    ///
    /// Returns a reference to the connection matrix used by this system dictionary.
    /// This is needed for UserDictionary initialization.
    ///
    /// # Returns
    /// * `ConnectionMatrix` - Shared reference to connection matrix
    pub fn connection_matrix(&self) -> ConnectionMatrix {
        self.ram_dict.connection_matrix()
    }

    /// Get character categories for a given character
    ///
    /// Returns all character categories that match the given character,
    /// including both primary and compatible categories.
    ///
    /// # Arguments
    /// * `ch` - Character to classify
    ///
    /// # Returns
    /// HashMap mapping category names to compatible category lists
    pub fn get_char_categories(&self, ch: char) -> HashMap<String, Vec<String>> {
        self.ram_dict.get_resource().get_char_categories(ch)
    }

    /// Check if unknown word processing should always be invoked for category
    ///
    /// # Arguments
    /// * `category` - Character category name
    ///
    /// # Returns
    /// True if unknown word processing should always be invoked
    pub fn unknown_invoked_always(&self, category: &str) -> bool {
        self.ram_dict
            .get_resource()
            .unknown_invoked_always(category)
    }

    pub fn unknown_invoked_always_id(&self, category: u16) -> bool {
        self.ram_dict
            .get_resource()
            .unknown_invoked_always_by_id(category)
    }

    /// Check if characters of this category should be grouped together
    ///
    /// # Arguments  
    /// * `category` - Character category name
    ///
    /// # Returns
    /// True if consecutive characters should be grouped
    pub fn unknown_grouping(&self, category: &str) -> bool {
        self.ram_dict.get_resource().unknown_grouping(category)
    }

    pub fn unknown_grouping_id(&self, category: u16) -> bool {
        self.ram_dict
            .get_resource()
            .unknown_grouping_by_id(category)
    }

    /// Get length constraint for unknown words of this category
    ///
    /// # Arguments
    /// * `category` - Character category name  
    ///
    /// # Returns
    /// Length constraint (-1 = no limit, positive = max length)
    pub fn unknown_length(&self, category: &str) -> i32 {
        self.ram_dict.get_resource().unknown_length(category)
    }

    pub fn unknown_length_id(&self, category: u16) -> usize {
        let len = self.ram_dict.get_resource().unknown_length_by_id(category);
        if len <= 0 { usize::MAX } else { len as usize }
    }

    /// Get unknown word entries for a character category
    ///
    /// Returns the list of unknown word templates for the given category.
    /// These templates define the morphological properties (cost, part-of-speech, etc.)
    /// for unknown words of this category.
    ///
    /// # Arguments
    /// * `category` - Character category name
    ///
    /// # Returns
    /// Option containing slice of unknown entries for this category
    pub fn get_unknown_entries(
        &self,
        category: &str,
    ) -> Option<&[crate::dictionary::UnknownEntry]> {
        self.ram_dict.get_resource().get_unknown_entries(category)
    }

    pub fn get_unknown_entries_by_id(&self, category: u16) -> Option<&[UnknownEntry]> {
        self.ram_dict
            .get_resource()
            .get_unknown_entries_by_id(category)
    }

    /// Get character categories for a given character (Result version)
    ///
    /// Returns the list of character categories that apply to the given character.
    /// This is used for unknown word processing to determine how to handle
    /// characters not found in the dictionary.
    ///
    /// # Arguments
    /// * `c` - Character to classify
    ///
    /// # Returns
    /// * `Ok(Vec<String>)` - Vector of category names that apply to this character
    /// * `Err(RunomeError)` - Error if character classification fails
    pub fn get_char_categories_result(&self, c: char) -> Result<Vec<String>, RunomeError> {
        let mut result = Vec::new();
        let mut found = false;

        for (category, compat_categories) in self.ram_dict.get_resource().iter_char_categories(c) {
            found = true;
            result.push(category);
            result.extend(compat_categories);
        }

        if !found {
            result.push("DEFAULT".to_string());
        }

        Ok(result)
    }

    pub fn get_char_category_ids(&self, c: char) -> Vec<u16> {
        self.ram_dict.get_resource().get_char_category_ids(c)
    }

    pub(crate) fn get_char_category_mask(&self, c: char) -> CategoryMask {
        self.ram_dict.get_resource().get_char_category_mask(c)
    }

    pub fn category_name_by_id(&self, category: u16) -> &str {
        self.ram_dict.get_resource().category_name_by_id(category)
    }

    pub fn category_id_by_name(&self, name: &str) -> Option<u16> {
        self.ram_dict.get_resource().category_id_by_name(name)
    }

    /// Get unknown word entries for a character category (Result version)
    ///
    /// Returns the list of unknown word templates for the given category.
    /// These templates define the morphological properties (cost, part-of-speech, etc.)
    /// for unknown words of this category.
    ///
    /// # Arguments
    /// * `category` - Character category name
    ///
    /// # Returns
    /// * `Ok(Vec<&UnknownEntry>)` - Vector of unknown word entries for this category
    /// * `Err(RunomeError)` - Error if category is not found
    pub fn get_unknown_entries_result(
        &self,
        category: &str,
    ) -> Result<&[UnknownEntry], RunomeError> {
        self.get_unknown_entries(category)
            .ok_or_else(|| RunomeError::DictValidationError {
                reason: format!("Unknown category: {}", category),
            })
    }

    /// Check if unknown word processing should always be invoked for this category (Result version)
    ///
    /// Returns true if unknown word processing should be performed even when
    /// dictionary entries are found. This is used for categories like numbers
    /// that may have both dictionary entries and unknown word processing.
    ///
    /// # Arguments
    /// * `category` - Character category name
    ///
    /// # Returns
    /// * `Ok(bool)` - True if unknown processing should always be invoked
    /// * `Err(RunomeError)` - Error if category is not found
    pub fn unknown_invoked_always_result(&self, category: &str) -> Result<bool, RunomeError> {
        Ok(self.unknown_invoked_always(category))
    }

    /// Check if unknown words of this category should be grouped together (Result version)
    ///
    /// Returns true if consecutive characters of the same category should be
    /// grouped into a single unknown word (e.g., "2009" instead of "2", "0", "0", "9").
    ///
    /// # Arguments
    /// * `category` - Character category name
    ///
    /// # Returns
    /// * `Ok(bool)` - True if characters should be grouped
    /// * `Err(RunomeError)` - Error if category is not found
    pub fn unknown_grouping_result(&self, category: &str) -> Result<bool, RunomeError> {
        Ok(self.unknown_grouping(category))
    }

    /// Get the maximum length for unknown words of this category (Result version)
    ///
    /// Returns the maximum number of characters that can be grouped together
    /// for unknown words of this category.
    ///
    /// # Arguments
    /// * `category` - Character category name
    ///
    /// # Returns
    /// * `Ok(usize)` - Maximum length for this category
    /// * `Err(RunomeError)` - Error if category is not found
    pub fn unknown_length_result(&self, category: &str) -> Result<usize, RunomeError> {
        let length = self.unknown_length(category);
        if length <= 0 {
            Ok(usize::MAX) // -1 or 0 means no limit
        } else {
            Ok(length as usize)
        }
    }
}

/// Implement Dictionary trait through delegation to RAMDictionary
impl Dictionary for SystemDictionary {
    fn lookup(&self, surface: &str) -> Result<Vec<&DictEntry>, RunomeError> {
        self.lookup(surface)
    }

    fn get_trans_cost(&self, left_id: u16, right_id: u16) -> Result<i16, RunomeError> {
        self.get_trans_cost(left_id, right_id)
    }

    fn lookup_into<'a>(
        &'a self,
        surface: &str,
        buffer: &mut Vec<&'a DictEntry>,
    ) -> Result<(), RunomeError> {
        self.ram_dict.lookup_into(surface, buffer)
    }

    fn lookup_into_with_index_buffer<'a>(
        &'a self,
        surface: &str,
        buffer: &mut Vec<&'a DictEntry>,
        index_buffer: &mut Vec<u64>,
    ) -> Result<(), RunomeError> {
        self.ram_dict
            .lookup_into_with_index_buffer(surface, buffer, index_buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn get_test_sysdic_path() -> PathBuf {
        PathBuf::from("sysdic")
    }

    #[test]
    fn test_system_dictionary_creation() {
        let sysdic_path = get_test_sysdic_path();
        if !sysdic_path.exists() {
            eprintln!(
                "Skipping test: sysdic directory not found at {:?}",
                sysdic_path
            );
            return;
        }

        let sys_dict_result = SystemDictionary::new(&sysdic_path);
        assert!(
            sys_dict_result.is_ok(),
            "Failed to create SystemDictionary: {:?}",
            sys_dict_result.err()
        );
    }

    #[test]
    fn test_singleton_consistency() {
        let sysdic_path = get_test_sysdic_path();
        if !sysdic_path.exists() {
            eprintln!(
                "Skipping test: sysdic directory not found at {:?}",
                sysdic_path
            );
            return;
        }

        let instance1 = SystemDictionary::instance();
        let instance2 = SystemDictionary::instance();

        assert!(instance1.is_ok(), "First instance creation should succeed");
        assert!(instance2.is_ok(), "Second instance creation should succeed");

        // Should be the same instance
        let inst1 = instance1.unwrap();
        let inst2 = instance2.unwrap();
        assert!(Arc::ptr_eq(&inst1, &inst2), "Instances should be the same");
    }

    #[test]
    fn test_lookup_delegation() {
        let sysdic_path = get_test_sysdic_path();
        if !sysdic_path.exists() {
            eprintln!(
                "Skipping test: sysdic directory not found at {:?}",
                sysdic_path
            );
            return;
        }

        let sys_dict = SystemDictionary::instance();
        assert!(sys_dict.is_ok(), "SystemDictionary creation should succeed");

        let sys_dict = sys_dict.unwrap();
        let lookup_result = sys_dict.lookup("東京");
        assert!(lookup_result.is_ok(), "Lookup should not fail");

        // Should return same results as direct RAMDictionary lookup
        let entries = lookup_result.unwrap();
        if !entries.is_empty() {
            for entry in entries {
                assert!(
                    !entry.part_of_speech.is_empty(),
                    "Part of speech should not be empty"
                );
            }
        }
    }

    #[test]
    fn test_get_trans_cost_delegation() {
        let sysdic_path = get_test_sysdic_path();
        if !sysdic_path.exists() {
            eprintln!(
                "Skipping test: sysdic directory not found at {:?}",
                sysdic_path
            );
            return;
        }

        let sys_dict = SystemDictionary::instance();
        assert!(sys_dict.is_ok(), "SystemDictionary creation should succeed");

        let sys_dict = sys_dict.unwrap();
        let cost_result = sys_dict.get_trans_cost(0, 0);
        assert!(cost_result.is_ok(), "get_trans_cost should work");
    }

    #[test]
    fn test_character_classification() {
        let sysdic_path = get_test_sysdic_path();
        if !sysdic_path.exists() {
            eprintln!(
                "Skipping test: sysdic directory not found at {:?}",
                sysdic_path
            );
            return;
        }

        let sys_dict = SystemDictionary::instance();
        assert!(sys_dict.is_ok(), "SystemDictionary creation should succeed");

        let sys_dict = sys_dict.unwrap();

        // Test character classification with real implementation
        let categories = sys_dict.get_char_categories('は');
        assert!(
            !categories.is_empty(),
            "Should have categories for hiragana"
        );

        // Test that some expected categories are present (or DEFAULT if not found)
        let has_hiragana_or_default =
            categories.contains_key("HIRAGANA") || categories.contains_key("DEFAULT");
        assert!(
            has_hiragana_or_default,
            "Should have HIRAGANA category or DEFAULT fallback"
        );

        // Test unknown word processing flags - behavior depends on actual character definitions
        // Just verify methods work without error
        let _ = sys_dict.unknown_invoked_always("HIRAGANA");
        let _ = sys_dict.unknown_grouping("HIRAGANA");
        let length = sys_dict.unknown_length("HIRAGANA");
        assert!(length >= -1, "Length should be valid (-1 or positive)");
    }

    #[test]
    fn test_character_classification_comprehensive() {
        let sysdic_path = get_test_sysdic_path();
        if !sysdic_path.exists() {
            eprintln!(
                "Skipping test: sysdic directory not found at {:?}",
                sysdic_path
            );
            return;
        }

        let sys_dict = SystemDictionary::instance();
        assert!(sys_dict.is_ok(), "SystemDictionary creation should succeed");
        let sys_dict = sys_dict.unwrap();

        // Test major Japanese character types
        let japanese_test_cases = [
            // Hiragana
            ('あ', "HIRAGANA", "basic hiragana"),
            ('か', "HIRAGANA", "hiragana ka"),
            ('ひ', "HIRAGANA", "hiragana hi"),
            ('ん', "HIRAGANA", "hiragana n"),
            // Katakana
            ('ア', "KATAKANA", "basic katakana"),
            ('カ', "KATAKANA", "katakana ka"),
            ('ヒ', "KATAKANA", "katakana hi"),
            ('ン', "KATAKANA", "katakana n"),
            // Kanji
            ('漢', "KANJI", "kanji character"),
            ('字', "KANJI", "ji character"),
            ('日', "KANJI", "nichi character"),
            ('本', "KANJI", "hon character"),
        ];

        for (ch, expected_category, description) in japanese_test_cases {
            let categories = sys_dict.get_char_categories(ch);
            assert!(
                !categories.is_empty(),
                "Character '{}' ({}) should have categories",
                ch,
                description
            );

            // Should have expected category or DEFAULT
            let has_expected_or_default =
                categories.contains_key(expected_category) || categories.contains_key("DEFAULT");
            assert!(
                has_expected_or_default,
                "Character '{}' ({}) should have {} category or DEFAULT fallback. Got: {:?}",
                ch,
                description,
                expected_category,
                categories.keys().collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn test_character_classification_ascii_and_symbols() {
        let sysdic_path = get_test_sysdic_path();
        if !sysdic_path.exists() {
            eprintln!(
                "Skipping test: sysdic directory not found at {:?}",
                sysdic_path
            );
            return;
        }

        let sys_dict = SystemDictionary::instance();
        assert!(sys_dict.is_ok(), "SystemDictionary creation should succeed");
        let sys_dict = sys_dict.unwrap();

        // Test ASCII and symbol characters
        let ascii_symbol_test_cases = [
            // ASCII letters
            ('A', vec!["ALPHA", "DEFAULT"], "ASCII uppercase A"),
            ('z', vec!["ALPHA", "DEFAULT"], "ASCII lowercase z"),
            // ASCII digits
            ('0', vec!["NUMERIC", "DEFAULT"], "ASCII digit 0"),
            ('9', vec!["NUMERIC", "DEFAULT"], "ASCII digit 9"),
            // Common symbols
            ('!', vec!["SYMBOL", "DEFAULT"], "exclamation mark"),
            ('?', vec!["SYMBOL", "DEFAULT"], "question mark"),
            (' ', vec!["SPACE", "DEFAULT"], "space character"),
            // Japanese punctuation
            ('、', vec!["SYMBOL", "DEFAULT"], "Japanese comma"),
            ('。', vec!["SYMBOL", "DEFAULT"], "Japanese period"),
        ];

        for (ch, possible_categories, description) in ascii_symbol_test_cases {
            let categories = sys_dict.get_char_categories(ch);
            assert!(
                !categories.is_empty(),
                "Character '{}' ({}) should have categories",
                ch,
                description
            );

            // Should have at least one of the possible categories
            let has_valid_category = possible_categories
                .iter()
                .any(|cat| categories.contains_key(*cat));
            assert!(
                has_valid_category,
                "Character '{}' ({}) should have one of {:?}. Got: {:?}",
                ch,
                description,
                possible_categories,
                categories.keys().collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn test_character_classification_numeric_variants() {
        let sysdic_path = get_test_sysdic_path();
        if !sysdic_path.exists() {
            eprintln!(
                "Skipping test: sysdic directory not found at {:?}",
                sysdic_path
            );
            return;
        }

        let sys_dict = SystemDictionary::instance();
        assert!(sys_dict.is_ok(), "SystemDictionary creation should succeed");
        let sys_dict = sys_dict.unwrap();

        // Test different types of numeric characters
        let numeric_test_cases = [
            // ASCII digits
            ('1', vec!["NUMERIC", "DEFAULT"], "ASCII digit 1"),
            ('5', vec!["NUMERIC", "DEFAULT"], "ASCII digit 5"),
            // Full-width digits
            ('１', vec!["NUMERIC", "DEFAULT"], "full-width digit 1"),
            ('５', vec!["NUMERIC", "DEFAULT"], "full-width digit 5"),
            // Japanese numerals (these might be KANJI + KANJINUMERIC)
            (
                '一',
                vec!["KANJI", "KANJINUMERIC", "DEFAULT"],
                "kanji numeral 1",
            ),
            (
                '五',
                vec!["KANJI", "KANJINUMERIC", "DEFAULT"],
                "kanji numeral 5",
            ),
            (
                '十',
                vec!["KANJI", "KANJINUMERIC", "DEFAULT"],
                "kanji numeral 10",
            ),
        ];

        for (ch, possible_categories, description) in numeric_test_cases {
            let categories = sys_dict.get_char_categories(ch);
            assert!(
                !categories.is_empty(),
                "Character '{}' ({}) should have categories",
                ch,
                description
            );

            // Should have at least one of the possible categories
            let has_valid_category = possible_categories
                .iter()
                .any(|cat| categories.contains_key(*cat));
            assert!(
                has_valid_category,
                "Character '{}' ({}) should have one of {:?}. Got: {:?}",
                ch,
                description,
                possible_categories,
                categories.keys().collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn test_character_classification_kanji_numeric_compatibility() {
        let sysdic_path = get_test_sysdic_path();
        if !sysdic_path.exists() {
            eprintln!(
                "Skipping test: sysdic directory not found at {:?}",
                sysdic_path
            );
            return;
        }

        let sys_dict = SystemDictionary::instance();
        assert!(sys_dict.is_ok(), "SystemDictionary creation should succeed");
        let sys_dict = sys_dict.unwrap();

        // Test that KANJINUMERIC characters also have KANJI compatibility
        let kanji_numeric_chars = ['一', '二', '三', '四', '五', '六', '七', '八', '九', '十'];

        for ch in kanji_numeric_chars {
            let categories = sys_dict.get_char_categories(ch);

            // If character has KANJINUMERIC category, it should also be compatible with KANJI
            if categories.contains_key("KANJINUMERIC") {
                let kanjinumeric_compat = categories.get("KANJINUMERIC").unwrap();
                assert!(
                    categories.contains_key("KANJI")
                        || kanjinumeric_compat.contains(&"KANJI".to_string()),
                    "Character '{}' with KANJINUMERIC should also have KANJI category or compatibility. Categories: {:?}",
                    ch,
                    categories
                );
            }
        }
    }

    #[test]
    fn test_unknown_word_processing_properties() {
        let sysdic_path = get_test_sysdic_path();
        if !sysdic_path.exists() {
            eprintln!(
                "Skipping test: sysdic directory not found at {:?}",
                sysdic_path
            );
            return;
        }

        let sys_dict = SystemDictionary::instance();
        assert!(sys_dict.is_ok(), "SystemDictionary creation should succeed");
        let sys_dict = sys_dict.unwrap();

        // Test unknown word processing properties for common categories
        let categories_to_test = [
            "HIRAGANA",
            "KATAKANA",
            "KANJI",
            "KANJINUMERIC",
            "ALPHA",
            "NUMERIC",
            "SYMBOL",
            "DEFAULT",
        ];

        for category in categories_to_test {
            // Test that all methods work without panicking
            let invoke_always = sys_dict.unknown_invoked_always(category);
            let grouping = sys_dict.unknown_grouping(category);
            let length = sys_dict.unknown_length(category);

            // Verify return values are reasonable
            assert!(
                (-1..=255).contains(&length),
                "Length for category '{}' should be between -1 and 255, got: {}",
                category,
                length
            );

            // Log the properties for debugging (will only show if test fails)
            if invoke_always || grouping || length != -1 {
                eprintln!(
                    "Category '{}': invoke_always={}, grouping={}, length={}",
                    category, invoke_always, grouping, length
                );
            }
        }
    }

    #[test]
    fn test_character_classification_consistency() {
        let sysdic_path = get_test_sysdic_path();
        if !sysdic_path.exists() {
            eprintln!(
                "Skipping test: sysdic directory not found at {:?}",
                sysdic_path
            );
            return;
        }

        let sys_dict = SystemDictionary::instance();
        assert!(sys_dict.is_ok(), "SystemDictionary creation should succeed");
        let sys_dict = sys_dict.unwrap();

        // Test that multiple calls return consistent results
        let test_chars = ['あ', 'ア', '漢', 'A', '1', '、'];

        for ch in test_chars {
            let first_result = sys_dict.get_char_categories(ch);
            let second_result = sys_dict.get_char_categories(ch);
            let third_result = sys_dict.get_char_categories(ch);

            assert_eq!(
                first_result, second_result,
                "Character '{}' classification should be consistent between calls",
                ch
            );
            assert_eq!(
                second_result, third_result,
                "Character '{}' classification should be consistent across multiple calls",
                ch
            );
        }
    }

    #[test]
    fn test_character_classification_edge_cases() {
        let sysdic_path = get_test_sysdic_path();
        if !sysdic_path.exists() {
            eprintln!(
                "Skipping test: sysdic directory not found at {:?}",
                sysdic_path
            );
            return;
        }

        let sys_dict = SystemDictionary::instance();
        assert!(sys_dict.is_ok(), "SystemDictionary creation should succeed");
        let sys_dict = sys_dict.unwrap();

        // Test boundary characters and edge cases
        let edge_case_chars = [
            // Unicode boundary characters
            ('\u{0000}', "null character"),
            ('\u{007F}', "DEL character"),
            ('\u{0080}', "first extended ASCII"),
            ('\u{00FF}', "last extended ASCII"),
            // Hiragana boundaries
            ('\u{3041}', "first hiragana (small a)"),
            ('\u{3096}', "last hiragana"),
            // Katakana boundaries
            ('\u{30A1}', "first katakana (small a)"),
            ('\u{30F6}', "last katakana"),
            // CJK boundaries
            ('\u{4E00}', "first CJK ideograph"),
            ('\u{9FFF}', "last CJK ideograph"),
            // Unusual but valid characters
            ('\u{3000}', "ideographic space"),
            ('\u{FEFF}', "zero-width no-break space"),
            ('\u{200B}', "zero-width space"),
            // Full-width variants
            ('Ａ', "full-width A"),
            ('０', "full-width 0"),
            ('！', "full-width exclamation"),
        ];

        for (ch, description) in edge_case_chars {
            let categories = sys_dict.get_char_categories(ch);

            // All characters should get at least some classification (even if DEFAULT)
            assert!(
                !categories.is_empty(),
                "Edge case character '{}' (U+{:04X}) ({}) should have at least one category",
                ch,
                ch as u32,
                description
            );

            // Verify that all category names are non-empty strings
            for (category_name, compat_categories) in &categories {
                assert!(
                    !category_name.is_empty(),
                    "Category name should not be empty for character '{}' ({})",
                    ch,
                    description
                );

                // Verify compatibility categories are also valid
                for compat in compat_categories {
                    assert!(
                        !compat.is_empty(),
                        "Compatibility category should not be empty for character '{}' ({})",
                        ch,
                        description
                    );
                }
            }
        }
    }

    #[test]
    fn test_character_classification_rare_unicode() {
        let sysdic_path = get_test_sysdic_path();
        if !sysdic_path.exists() {
            eprintln!(
                "Skipping test: sysdic directory not found at {:?}",
                sysdic_path
            );
            return;
        }

        let sys_dict = SystemDictionary::instance();
        assert!(sys_dict.is_ok(), "SystemDictionary creation should succeed");
        let sys_dict = sys_dict.unwrap();

        // Test rare but potentially encountered Unicode characters
        let rare_unicode_chars = [
            // Extended punctuation
            ('\u{2026}', "horizontal ellipsis"),
            ('\u{2014}', "em dash"),
            ('\u{2013}', "en dash"),
            // Mathematical symbols
            ('\u{221E}', "infinity symbol"),
            ('\u{2260}', "not equal to"),
            ('\u{00B1}', "plus-minus sign"),
            // Currency symbols
            ('\u{00A5}', "yen sign"),
            ('\u{20AC}', "euro sign"),
            ('\u{0024}', "dollar sign"),
            // Arrows
            ('\u{2190}', "leftwards arrow"),
            ('\u{2192}', "rightwards arrow"),
            // Emoji (basic)
            ('\u{1F600}', "grinning face emoji"),
            ('\u{1F44D}', "thumbs up emoji"),
        ];

        for (ch, description) in rare_unicode_chars {
            let categories = sys_dict.get_char_categories(ch);

            // Should handle rare characters gracefully
            assert!(
                !categories.is_empty(),
                "Rare Unicode character '{}' (U+{:04X}) ({}) should be classified",
                ch,
                ch as u32,
                description
            );

            // Most rare characters should fall back to DEFAULT if not specifically categorized
            if !categories.keys().any(|k| k != "DEFAULT") {
                assert!(
                    categories.contains_key("DEFAULT"),
                    "Rare character '{}' ({}) should at least have DEFAULT category",
                    ch,
                    description
                );
            }
        }
    }

    #[test]
    fn test_character_classification_combining_characters() {
        let sysdic_path = get_test_sysdic_path();
        if !sysdic_path.exists() {
            eprintln!(
                "Skipping test: sysdic directory not found at {:?}",
                sysdic_path
            );
            return;
        }

        let sys_dict = SystemDictionary::instance();
        assert!(sys_dict.is_ok(), "SystemDictionary creation should succeed");
        let sys_dict = sys_dict.unwrap();

        // Test combining characters (accents, diacritics)
        let combining_chars = [
            ('\u{0300}', "combining grave accent"),
            ('\u{0301}', "combining acute accent"),
            ('\u{0302}', "combining circumflex accent"),
            ('\u{0303}', "combining tilde"),
            ('\u{0304}', "combining macron"),
            ('\u{0308}', "combining diaeresis"),
            ('\u{030A}', "combining ring above"),
        ];

        for (ch, description) in combining_chars {
            let categories = sys_dict.get_char_categories(ch);

            // Combining characters should be classified (likely as DEFAULT or special category)
            assert!(
                !categories.is_empty(),
                "Combining character '{}' (U+{:04X}) ({}) should be classified",
                ch,
                ch as u32,
                description
            );
        }
    }

    #[test]
    fn test_character_classification_surrogate_handling() {
        let sysdic_path = get_test_sysdic_path();
        if !sysdic_path.exists() {
            eprintln!(
                "Skipping test: sysdic directory not found at {:?}",
                sysdic_path
            );
            return;
        }

        let sys_dict = SystemDictionary::instance();
        assert!(sys_dict.is_ok(), "SystemDictionary creation should succeed");
        let sys_dict = sys_dict.unwrap();

        // Test characters that require surrogate pairs in UTF-16
        // These are valid Unicode scalar values that Rust char can represent
        let high_unicode_chars = [
            ('\u{10000}', "first supplementary plane character"),
            ('\u{1F300}', "cyclone emoji"),
            ('\u{1F680}', "rocket emoji"),
            ('\u{20000}', "CJK extension B character"),
        ];

        for (ch, description) in high_unicode_chars {
            let categories = sys_dict.get_char_categories(ch);

            // High Unicode characters should be handled gracefully
            assert!(
                !categories.is_empty(),
                "High Unicode character '{}' (U+{:04X}) ({}) should be classified",
                ch,
                ch as u32,
                description
            );
        }
    }

    #[test]
    fn test_debug_fst_lookup() {
        // Simple debug test to check FST functionality with new encoding
        let sysdic_path = get_test_sysdic_path();
        if !sysdic_path.exists() {
            eprintln!(
                "Skipping test: sysdic directory not found at {:?}",
                sysdic_path
            );
            return;
        }

        let sys_dict = SystemDictionary::instance();
        assert!(sys_dict.is_ok(), "SystemDictionary creation should succeed");
        let sys_dict = sys_dict.unwrap();

        // Test a simple lookup
        let test_words = ["形態素", "すもも", "東京"];

        for word in test_words {
            eprintln!("=== Testing lookup for: {} ===", word);

            let result = sys_dict.lookup(word);
            match result {
                Ok(entries) => {
                    eprintln!("Success! Found {} entries", entries.len());
                    for (i, entry) in entries.iter().enumerate() {
                        eprintln!(
                            "  [{}]: surface='{}', left_id={}, right_id={}, cost={}",
                            i, entry.surface, entry.left_id, entry.right_id, entry.cost
                        );
                    }
                }
                Err(e) => {
                    eprintln!("Lookup failed: {:?}", e);
                }
            }
            eprintln!();
        }
    }

    #[test]
    fn test_dictionary_ipadic() {
        // Equivalent to Python TestSystemDictionary.test_dictionary_ipadic()
        let sysdic_path = get_test_sysdic_path();
        if !sysdic_path.exists() {
            eprintln!(
                "Skipping test: sysdic directory not found at {:?}",
                sysdic_path
            );
            return;
        }

        let sys_dict = SystemDictionary::instance();
        assert!(sys_dict.is_ok(), "SystemDictionary creation should succeed");
        let sys_dict = sys_dict.unwrap();

        // Test 1: Dictionary lookup - equivalent to sys_dic.lookup('形態素'.encode('utf-8'), matcher)
        // Note: Python test expects 7 entries, our Rust implementation finds 3 due to different dictionary handling
        let morpheme_entries = sys_dict.lookup("形態素");
        assert!(
            morpheme_entries.is_ok(),
            "Lookup for '形態素' should succeed"
        );
        let entries = morpheme_entries.unwrap();

        // Debug: Show what entries we're actually getting
        eprintln!("=== Dictionary lookup for '形態素' ===");
        eprintln!("Found {} entries:", entries.len());
        for (i, entry) in entries.iter().enumerate() {
            eprintln!(
                "  [{}]: surface='{}', left_id={}, right_id={}, cost={}, pos='{}'",
                i, entry.surface, entry.left_id, entry.right_id, entry.cost, entry.part_of_speech
            );
        }

        // Verify we get expected entries (Python gets 7, we should get them too with enhanced FST encoding)
        assert!(
            entries.len() == 7,
            "Expected 7 entries for '形態素', got {}",
            entries.len()
        );

        // Verify that we find the exact match "形態素"
        let exact_match = entries.iter().find(|e| e.surface == "形態素");
        assert!(
            exact_match.is_some(),
            "Should find exact match for '形態素'"
        );

        // Verify entries have valid structure
        for entry in &entries {
            assert!(
                !entry.surface.is_empty(),
                "Entry surface should not be empty"
            );
            assert!(
                !entry.part_of_speech.is_empty(),
                "Entry part_of_speech should not be empty"
            );
            assert!(entry.left_id > 0, "Entry should have valid left_id");
            assert!(entry.right_id > 0, "Entry should have valid right_id");
        }

        // Test 2: Transition cost - equivalent to sys_dic.get_trans_cost(0, 1)
        let trans_cost = sys_dict.get_trans_cost(0, 1);
        assert!(trans_cost.is_ok(), "Getting transition cost should succeed");
        assert_eq!(
            trans_cost.unwrap(),
            1,
            "Transition cost from 0 to 1 should be 1 like Python test"
        );

        // Test 3: Character classification tests - equivalent to sys_dic.get_char_categories()
        // Each test verifies specific character categories match Python expectations

        // Hiragana: 'は' → {'HIRAGANA': []}
        let hiragana_cats = sys_dict.get_char_categories('は');
        assert!(
            hiragana_cats.contains_key("HIRAGANA"),
            "Character 'は' should have HIRAGANA category"
        );
        assert_eq!(
            hiragana_cats.get("HIRAGANA").unwrap(),
            &Vec::<String>::new(),
            "HIRAGANA category should have empty compatibility list"
        );

        // Katakana: 'ハ' → {'KATAKANA': []}
        let katakana_cats = sys_dict.get_char_categories('ハ');
        assert!(
            katakana_cats.contains_key("KATAKANA"),
            "Character 'ハ' should have KATAKANA category"
        );
        assert_eq!(
            katakana_cats.get("KATAKANA").unwrap(),
            &Vec::<String>::new(),
            "KATAKANA category should have empty compatibility list"
        );

        // Half-width Katakana: 'ﾊ' → {'KATAKANA': []}
        let halfwidth_katakana_cats = sys_dict.get_char_categories('ﾊ');
        assert!(
            halfwidth_katakana_cats.contains_key("KATAKANA"),
            "Character 'ﾊ' should have KATAKANA category"
        );

        // Kanji: '葉' → {'KANJI': []}
        let kanji_cats = sys_dict.get_char_categories('葉');
        assert!(
            kanji_cats.contains_key("KANJI"),
            "Character '葉' should have KANJI category"
        );
        assert_eq!(
            kanji_cats.get("KANJI").unwrap(),
            &Vec::<String>::new(),
            "KANJI category should have empty compatibility list"
        );

        // ASCII alphabetic: 'C' → {'ALPHA': []}
        let alpha_cats = sys_dict.get_char_categories('C');
        assert!(
            alpha_cats.contains_key("ALPHA"),
            "Character 'C' should have ALPHA category"
        );

        // Full-width alphabetic: 'Ｃ' → {'ALPHA': []}
        let fullwidth_alpha_cats = sys_dict.get_char_categories('Ｃ');
        assert!(
            fullwidth_alpha_cats.contains_key("ALPHA"),
            "Character 'Ｃ' should have ALPHA category"
        );

        // Symbol: '#' → {'SYMBOL': []}
        let symbol_cats = sys_dict.get_char_categories('#');
        assert!(
            symbol_cats.contains_key("SYMBOL"),
            "Character '#' should have SYMBOL category"
        );

        // Full-width symbol: '＃' → {'SYMBOL': []}
        let fullwidth_symbol_cats = sys_dict.get_char_categories('＃');
        assert!(
            fullwidth_symbol_cats.contains_key("SYMBOL"),
            "Character '＃' should have SYMBOL category"
        );

        // Numeric: '5' → {'NUMERIC': []}
        let numeric_cats = sys_dict.get_char_categories('5');
        assert!(
            numeric_cats.contains_key("NUMERIC"),
            "Character '5' should have NUMERIC category"
        );

        // Full-width numeric: '５' → {'NUMERIC': []}
        let fullwidth_numeric_cats = sys_dict.get_char_categories('５');
        assert!(
            fullwidth_numeric_cats.contains_key("NUMERIC"),
            "Character '５' should have NUMERIC category"
        );

        // Kanji numeric: '五' → {'KANJI': [], 'KANJINUMERIC': ['KANJI']}
        let kanji_numeric_cats = sys_dict.get_char_categories('五');
        assert!(
            kanji_numeric_cats.contains_key("KANJI"),
            "Character '五' should have KANJI category"
        );
        assert!(
            kanji_numeric_cats.contains_key("KANJINUMERIC"),
            "Character '五' should have KANJINUMERIC category"
        );
        // KANJINUMERIC should have KANJI in compatibility list
        let kanjinumeric_compat = kanji_numeric_cats.get("KANJINUMERIC").unwrap();
        assert!(
            kanjinumeric_compat.contains(&"KANJI".to_string()),
            "KANJINUMERIC category should have KANJI in compatibility list"
        );

        // Greek: 'Γ' → {'GREEK': []}
        let greek_cats = sys_dict.get_char_categories('Γ');
        assert!(
            greek_cats.contains_key("GREEK"),
            "Character 'Γ' should have GREEK category"
        );

        // Cyrillic: 'Б' → {'CYRILLIC': []}
        let cyrillic_cats = sys_dict.get_char_categories('Б');
        assert!(
            cyrillic_cats.contains_key("CYRILLIC"),
            "Character 'Б' should have CYRILLIC category"
        );

        // Default category for variant kanji: '𠮷' → {'DEFAULT': []}
        let variant_kanji_cats = sys_dict.get_char_categories('𠮷');
        assert!(
            variant_kanji_cats.contains_key("DEFAULT"),
            "Character '𠮷' should have DEFAULT category"
        );

        // Default category for Korean: '한' → {'DEFAULT': []}
        let korean_cats = sys_dict.get_char_categories('한');
        assert!(
            korean_cats.contains_key("DEFAULT"),
            "Character '한' should have DEFAULT category"
        );

        // Test 4: Unknown word processing properties
        // equivalent to sys_dic.unknown_invoked_always(), unknown_grouping(), unknown_length()

        // ALPHA characters should always invoke unknown word processing
        assert!(
            sys_dict.unknown_invoked_always("ALPHA"),
            "ALPHA category should always invoke unknown word processing"
        );

        // KANJI characters should not always invoke unknown word processing
        assert!(
            !sys_dict.unknown_invoked_always("KANJI"),
            "KANJI category should not always invoke unknown word processing"
        );

        // NUMERIC characters should be grouped
        assert!(
            sys_dict.unknown_grouping("NUMERIC"),
            "NUMERIC category should enable grouping"
        );

        // KANJI characters should not be grouped
        assert!(
            !sys_dict.unknown_grouping("KANJI"),
            "KANJI category should not enable grouping"
        );

        // HIRAGANA unknown word length should be 2
        assert_eq!(
            sys_dict.unknown_length("HIRAGANA"),
            2,
            "HIRAGANA category should have unknown word length of 2"
        );
    }
}
