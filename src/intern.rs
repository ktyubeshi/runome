/// String interning module for common morphological values
///
/// This module provides static references to frequently used strings to eliminate
/// repeated allocations during tokenization. Based on analysis of the codebase:
/// - "*" appears 133 times as placeholder for missing morphological data
/// - "" appears 39 times for empty values  
/// - Character categories appear 100+ times during unknown word processing
/// - BOS/EOS markers used once per sentence but created frequently

/// Tier 1: Critical placeholders and sentinels (highest frequency)
pub const ASTERISK: &str = "*";
pub const EMPTY: &str = "";
pub const BOS_SURFACE: &str = "__BOS__";
pub const EOS_SURFACE: &str = "__EOS__";

/// Tier 1: Character categories (used heavily in unknown word processing)
pub const CHAR_CATEGORY_DEFAULT: &str = "DEFAULT";
pub const CHAR_CATEGORY_KANJI: &str = "KANJI";
pub const CHAR_CATEGORY_HIRAGANA: &str = "HIRAGANA";
pub const CHAR_CATEGORY_KATAKANA: &str = "KATAKANA";
pub const CHAR_CATEGORY_NUMERIC: &str = "NUMERIC";
pub const CHAR_CATEGORY_KANJINUMERIC: &str = "KANJINUMERIC";
pub const CHAR_CATEGORY_SYMBOL: &str = "SYMBOL";
pub const CHAR_CATEGORY_ALPHA: &str = "ALPHA";

/// Tier 2: Common part-of-speech patterns
pub const POS_NOUN_GENERAL: &str = "名詞,一般,*,*,*,*";
pub const POS_NOUN_GENERAL_PARTIAL: &str = "名詞,一般";
pub const POS_NOUN_COMPOUND: &str = "名詞,複合,*,*";
pub const POS_NOUN_PROPER: &str = "名詞,固有名詞";
pub const POS_PARTICLE: &str = "助詞";
pub const POS_NOUN: &str = "名詞";

/// Tier 2: Common separators and formatting
pub const COMMA_SPACE: &str = ", ";
pub const PIPE_SPACE: &str = " | ";

use std::borrow::Cow;

/// Helper function to get interned string if available, otherwise clone
/// This provides a migration path for gradually adopting string interning
pub fn intern_or_clone(s: &str) -> String {
    match s {
        // Tier 1: Critical patterns
        "*" => ASTERISK.to_string(),
        "" => EMPTY.to_string(),
        "__BOS__" => BOS_SURFACE.to_string(),
        "__EOS__" => EOS_SURFACE.to_string(),

        // Character categories
        "DEFAULT" => CHAR_CATEGORY_DEFAULT.to_string(),
        "KANJI" => CHAR_CATEGORY_KANJI.to_string(),
        "HIRAGANA" => CHAR_CATEGORY_HIRAGANA.to_string(),
        "KATAKANA" => CHAR_CATEGORY_KATAKANA.to_string(),
        "NUMERIC" => CHAR_CATEGORY_NUMERIC.to_string(),
        "KANJINUMERIC" => CHAR_CATEGORY_KANJINUMERIC.to_string(),
        "SYMBOL" => CHAR_CATEGORY_SYMBOL.to_string(),
        "ALPHA" => CHAR_CATEGORY_ALPHA.to_string(),

        // Part-of-speech patterns
        "名詞,一般,*,*,*,*" => POS_NOUN_GENERAL.to_string(),
        "名詞,一般" => POS_NOUN_GENERAL_PARTIAL.to_string(),
        "名詞,複合,*,*" => POS_NOUN_COMPOUND.to_string(),
        "名詞,固有名詞" => POS_NOUN_PROPER.to_string(),
        "助詞" => POS_PARTICLE.to_string(),
        "名詞" => POS_NOUN.to_string(),

        // Separators
        ", " => COMMA_SPACE.to_string(),
        " | " => PIPE_SPACE.to_string(),

        // Not found in intern table, clone as usual
        _ => s.to_string(),
    }
}

/// Helper function to get interned string reference if available
/// Returns None if string is not in the intern table
pub fn intern_ref(s: &str) -> Option<&'static str> {
    match s {
        // Tier 1: Critical patterns
        "*" => Some(ASTERISK),
        "" => Some(EMPTY),
        "__BOS__" => Some(BOS_SURFACE),
        "__EOS__" => Some(EOS_SURFACE),

        // Character categories
        "DEFAULT" => Some(CHAR_CATEGORY_DEFAULT),
        "KANJI" => Some(CHAR_CATEGORY_KANJI),
        "HIRAGANA" => Some(CHAR_CATEGORY_HIRAGANA),
        "KATAKANA" => Some(CHAR_CATEGORY_KATAKANA),
        "NUMERIC" => Some(CHAR_CATEGORY_NUMERIC),
        "KANJINUMERIC" => Some(CHAR_CATEGORY_KANJINUMERIC),
        "SYMBOL" => Some(CHAR_CATEGORY_SYMBOL),
        "ALPHA" => Some(CHAR_CATEGORY_ALPHA),

        // Part-of-speech patterns
        "名詞,一般,*,*,*,*" => Some(POS_NOUN_GENERAL),
        "名詞,一般" => Some(POS_NOUN_GENERAL_PARTIAL),
        "名詞,複合,*,*" => Some(POS_NOUN_COMPOUND),
        "名詞,固有名詞" => Some(POS_NOUN_PROPER),
        "助詞" => Some(POS_PARTICLE),
        "名詞" => Some(POS_NOUN),

        // Separators
        ", " => Some(COMMA_SPACE),
        " | " => Some(PIPE_SPACE),

        // Not found in intern table
        _ => None,
    }
}

/// Get Cow<str> with zero-copy for interned strings, owned for others
/// This is the preferred method for Token fields to enable zero-copy optimization
pub fn intern_or_cow(s: &str) -> Cow<'static, str> {
    match s {
        // Tier 1: Critical patterns - return static references (zero-copy)
        "*" => Cow::Borrowed(ASTERISK),
        "" => Cow::Borrowed(EMPTY),
        "__BOS__" => Cow::Borrowed(BOS_SURFACE),
        "__EOS__" => Cow::Borrowed(EOS_SURFACE),

        // Character categories - zero-copy static references
        "DEFAULT" => Cow::Borrowed(CHAR_CATEGORY_DEFAULT),
        "KANJI" => Cow::Borrowed(CHAR_CATEGORY_KANJI),
        "HIRAGANA" => Cow::Borrowed(CHAR_CATEGORY_HIRAGANA),
        "KATAKANA" => Cow::Borrowed(CHAR_CATEGORY_KATAKANA),
        "NUMERIC" => Cow::Borrowed(CHAR_CATEGORY_NUMERIC),
        "KANJINUMERIC" => Cow::Borrowed(CHAR_CATEGORY_KANJINUMERIC),
        "SYMBOL" => Cow::Borrowed(CHAR_CATEGORY_SYMBOL),
        "ALPHA" => Cow::Borrowed(CHAR_CATEGORY_ALPHA),

        // Part-of-speech patterns - zero-copy static references
        "名詞,一般,*,*,*,*" => Cow::Borrowed(POS_NOUN_GENERAL),
        "名詞,一般" => Cow::Borrowed(POS_NOUN_GENERAL_PARTIAL),
        "名詞,複合,*,*" => Cow::Borrowed(POS_NOUN_COMPOUND),
        "名詞,固有名詞" => Cow::Borrowed(POS_NOUN_PROPER),
        "助詞" => Cow::Borrowed(POS_PARTICLE),
        "名詞" => Cow::Borrowed(POS_NOUN),

        // Separators - zero-copy static references
        ", " => Cow::Borrowed(COMMA_SPACE),
        " | " => Cow::Borrowed(PIPE_SPACE),

        // Not found in intern table - must clone (owned)
        _ => Cow::Owned(s.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intern_constants() {
        assert_eq!(ASTERISK, "*");
        assert_eq!(EMPTY, "");
        assert_eq!(BOS_SURFACE, "__BOS__");
        assert_eq!(EOS_SURFACE, "__EOS__");
    }

    #[test]
    fn test_intern_or_clone() {
        // Test interned values
        assert_eq!(intern_or_clone("*"), "*");
        assert_eq!(intern_or_clone(""), "");
        assert_eq!(intern_or_clone("DEFAULT"), "DEFAULT");
        assert_eq!(intern_or_clone("名詞,一般,*,*,*,*"), "名詞,一般,*,*,*,*");

        // Test non-interned values
        assert_eq!(intern_or_clone("random_string"), "random_string");
    }

    #[test]
    fn test_intern_ref() {
        // Test interned values return Some
        assert!(intern_ref("*").is_some());
        assert!(intern_ref("").is_some());
        assert!(intern_ref("DEFAULT").is_some());

        // Test non-interned values return None
        assert!(intern_ref("random_string").is_none());
    }

    #[test]
    fn test_character_categories() {
        let categories = [
            "DEFAULT",
            "KANJI",
            "HIRAGANA",
            "KATAKANA",
            "NUMERIC",
            "KANJINUMERIC",
            "SYMBOL",
            "ALPHA",
        ];

        for category in &categories {
            assert!(
                intern_ref(category).is_some(),
                "Category {} not interned",
                category
            );
        }
    }

    #[test]
    fn test_pos_patterns() {
        let patterns = [
            "名詞,一般,*,*,*,*",
            "名詞,一般",
            "名詞,複合,*,*",
            "名詞,固有名詞",
            "助詞",
            "名詞",
        ];

        for pattern in &patterns {
            assert!(
                intern_ref(pattern).is_some(),
                "POS pattern {} not interned",
                pattern
            );
        }
    }

    #[test]
    fn test_intern_or_cow() {
        use std::borrow::Cow;

        // Test interned values return Borrowed
        match intern_or_cow("*") {
            Cow::Borrowed(s) => assert_eq!(s, "*"),
            Cow::Owned(_) => panic!("Should be borrowed"),
        }

        match intern_or_cow("DEFAULT") {
            Cow::Borrowed(s) => assert_eq!(s, "DEFAULT"),
            Cow::Owned(_) => panic!("Should be borrowed"),
        }

        // Test non-interned values return Owned
        match intern_or_cow("random_string") {
            Cow::Owned(s) => assert_eq!(s, "random_string"),
            Cow::Borrowed(_) => panic!("Should be owned"),
        }
    }

    #[test]
    fn test_intern_or_cow_zero_copy() {
        // Verify that interned strings are truly zero-copy
        let cow_asterisk = intern_or_cow("*");
        match cow_asterisk {
            Cow::Borrowed(s) => {
                assert_eq!(s, "*");
                // Verify it's the same as our constant (content equality is sufficient)
                assert_eq!(s, ASTERISK);
            }
            Cow::Owned(_) => panic!("Asterisk should be borrowed"),
        }

        // Test that non-interned strings are owned
        let cow_random = intern_or_cow("random_string");
        match cow_random {
            Cow::Owned(s) => assert_eq!(s, "random_string"),
            Cow::Borrowed(_) => panic!("Random string should be owned"),
        }
    }
}
