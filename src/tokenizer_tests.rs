use crate::lattice::NodeType;
use crate::tokenizer::{TokenizeResult, Tokenizer};

/// Segmentation tests module - tests for basic tokenization functionality
pub mod segmentation_tests {
    use super::*;

    /// Helper function to check token properties (equivalent to Python's _check_token)
    fn check_token(
        token: &TokenizeResult,
        expected_surface: &str,
        expected_detail: &str,
        expected_node_type: NodeType,
    ) {
        match token {
            TokenizeResult::Token(token) => {
                // Check surface form
                assert_eq!(token.surface(), expected_surface, "Surface mismatch");

                // Check combined detail string (part_of_speech,infl_type,infl_form,base_form,reading,phonetic)
                let actual_detail = format!(
                    "{},{},{},{},{},{}",
                    token.part_of_speech(),
                    token.infl_type(),
                    token.infl_form(),
                    token.base_form(),
                    token.reading(),
                    token.phonetic()
                );
                assert_eq!(
                    actual_detail, expected_detail,
                    "Detail mismatch for '{}'",
                    expected_surface
                );

                // Check string representation (surface + tab + detail)
                let expected_str = format!("{}\t{}", expected_surface, expected_detail);
                assert_eq!(
                    format!("{}", token),
                    expected_str,
                    "String representation mismatch"
                );

                // Check node type
                assert_eq!(
                    token.node_type(),
                    expected_node_type,
                    "Node type mismatch for '{}'",
                    expected_surface
                );
            }
            TokenizeResult::Surface(_) => {
                panic!("Expected Token but got Surface for '{}'", expected_surface);
            }
        }
    }

    #[test]
    fn test_tokenize_basic() {
        // Equivalent to Python's TestTokenizer.test_tokenize_nommap()
        // Tests basic tokenization with the classic "すもももももももものうち" example
        let tokenizer = Tokenizer::new(None, None);
        if tokenizer.is_err() {
            eprintln!("Skipping test: SystemDictionary not available");
            return;
        }
        let tokenizer = tokenizer.unwrap();

        let text = "すもももももももものうち";
        let results: Result<Vec<_>, _> = tokenizer.tokenize(text, None, None).collect();

        assert!(results.is_ok(), "Tokenization should succeed");
        let tokens = results.unwrap();

        // Should produce exactly 7 tokens
        assert_eq!(tokens.len(), 7, "Expected 7 tokens for '{}'", text);

        // Validate each token matches expected structure
        // ✅ FIXED: Now correctly uses SysDict node type for dictionary entries
        // ✅ FIXED: Now correctly extracts reading and phonetic fields from dictionary
        check_token(
            &tokens[0],
            "すもも",
            "名詞,一般,*,*,*,*,すもも,スモモ,スモモ",
            NodeType::SysDict,
        );
        check_token(
            &tokens[1],
            "も",
            "助詞,係助詞,*,*,*,*,も,モ,モ",
            NodeType::SysDict,
        );
        check_token(
            &tokens[2],
            "もも",
            "名詞,一般,*,*,*,*,もも,モモ,モモ",
            NodeType::SysDict,
        );
        check_token(
            &tokens[3],
            "も",
            "助詞,係助詞,*,*,*,*,も,モ,モ",
            NodeType::SysDict,
        );
        check_token(
            &tokens[4],
            "もも",
            "名詞,一般,*,*,*,*,もも,モモ,モモ",
            NodeType::SysDict,
        );
        check_token(
            &tokens[5],
            "の",
            "助詞,連体化,*,*,*,*,の,ノ,ノ",
            NodeType::SysDict,
        );
        check_token(
            &tokens[6],
            "うち",
            "名詞,非自立,副詞可能,*,*,*,うち,ウチ,ウチ",
            NodeType::SysDict,
        );
    }

    #[test]
    fn test_tokenize_mixed_known_unknown() {
        // Equivalent to Python's TestTokenizer.test_tokenize2()
        // Tests tokenization of text with both known and unknown characters
        let tokenizer = Tokenizer::new(None, None);
        if tokenizer.is_err() {
            eprintln!("Skipping test: SystemDictionary not available");
            return;
        }
        let tokenizer = tokenizer.unwrap();

        // Test case 1: Mixed known/unknown characters - '𠮷野屋'
        // 𠮷 is a rare kanji variant (U+20BB7) that should be unknown
        // 野 and 屋 should be found in the dictionary
        let text = "𠮷野屋";
        let results: Result<Vec<_>, _> = tokenizer.tokenize(text, None, None).collect();

        assert!(results.is_ok(), "Tokenization should succeed");
        let tokens = results.unwrap();

        assert_eq!(tokens.len(), 3, "Expected 3 tokens for '{}'", text);

        // 𠮷 should be unknown (rare kanji variant)
        check_token(
            &tokens[0],
            "𠮷",
            "記号,一般,*,*,*,*,𠮷,*,*",
            NodeType::Unknown,
        );

        // 野 should be in dictionary
        check_token(
            &tokens[1],
            "野",
            "名詞,一般,*,*,*,*,野,ノ,ノ",
            NodeType::SysDict,
        );

        // 屋 should be in dictionary
        check_token(
            &tokens[2],
            "屋",
            "名詞,接尾,一般,*,*,*,屋,ヤ,ヤ",
            NodeType::SysDict,
        );

        // Test case 2: Foreign text - Korean '한국어'
        // Should be treated as a single unknown token
        let text = "한국어";
        let results: Result<Vec<_>, _> = tokenizer.tokenize(text, None, None).collect();

        assert!(results.is_ok(), "Tokenization should succeed");
        let tokens = results.unwrap();

        assert_eq!(tokens.len(), 1, "Expected 1 token for '{}'", text);

        // Korean text should be unknown
        check_token(
            &tokens[0],
            "한국어",
            "記号,一般,*,*,*,*,한국어,*,*",
            NodeType::Unknown,
        );
    }

    #[test]
    fn test_tokenize_unknown() {
        // Equivalent to Python's TestTokenizer.test_tokenize_unknown()
        // Tests tokenization of text with various unknown word types (numbers, English, etc.)
        let tokenizer = Tokenizer::new(None, None);
        if tokenizer.is_err() {
            eprintln!("Skipping test: SystemDictionary not available");
            return;
        }
        let tokenizer = tokenizer.unwrap();

        // Test case 1: Date text with numbers - '2009年10月16日'
        // Numbers should be unknown, date markers should be in dictionary
        let text = "2009年10月16日";
        let results: Result<Vec<_>, _> = tokenizer.tokenize(text, None, None).collect();

        assert!(results.is_ok(), "Tokenization should succeed");
        let tokens = results.unwrap();

        assert_eq!(tokens.len(), 6, "Expected 6 tokens for '{}'", text);

        // Numbers should be unknown
        check_token(
            &tokens[0],
            "2009",
            "名詞,数,*,*,*,*,2009,*,*",
            NodeType::Unknown,
        );

        // Date markers should be in dictionary
        check_token(
            &tokens[1],
            "年",
            "名詞,接尾,助数詞,*,*,*,年,ネン,ネン",
            NodeType::SysDict,
        );

        check_token(
            &tokens[2],
            "10",
            "名詞,数,*,*,*,*,10,*,*",
            NodeType::Unknown,
        );

        check_token(
            &tokens[3],
            "月",
            "名詞,一般,*,*,*,*,月,ツキ,ツキ",
            NodeType::SysDict,
        );

        check_token(
            &tokens[4],
            "16",
            "名詞,数,*,*,*,*,16,*,*",
            NodeType::Unknown,
        );

        check_token(
            &tokens[5],
            "日",
            "名詞,接尾,助数詞,*,*,*,日,ニチ,ニチ",
            NodeType::SysDict,
        );

        // Test case 2: Mixed Japanese/English text - 'マルチメディア放送（VHF-HIGH帯）「モバキャス」'
        // Tests various punctuation, English words, and compound words
        let text = "マルチメディア放送（VHF-HIGH帯）「モバキャス」";
        let results: Result<Vec<_>, _> = tokenizer.tokenize(text, None, None).collect();

        assert!(results.is_ok(), "Tokenization should succeed");
        let tokens = results.unwrap();

        assert_eq!(tokens.len(), 11, "Expected 11 tokens for '{}'", text);

        // Japanese compound word in dictionary
        check_token(
            &tokens[0],
            "マルチメディア",
            "名詞,一般,*,*,*,*,マルチメディア,マルチメディア,マルチメディア",
            NodeType::SysDict,
        );

        // Japanese word in dictionary
        check_token(
            &tokens[1],
            "放送",
            "名詞,サ変接続,*,*,*,*,放送,ホウソウ,ホーソー",
            NodeType::SysDict,
        );

        // Punctuation in dictionary
        check_token(
            &tokens[2],
            "（",
            "記号,括弧開,*,*,*,*,（,（,（",
            NodeType::SysDict,
        );

        // English abbreviation - unknown
        check_token(
            &tokens[3],
            "VHF",
            "名詞,固有名詞,組織,*,*,*,VHF,*,*",
            NodeType::Unknown,
        );

        // Hyphen - unknown
        check_token(
            &tokens[4],
            "-",
            "名詞,サ変接続,*,*,*,*,-,*,*",
            NodeType::Unknown,
        );

        // English word - unknown
        check_token(
            &tokens[5],
            "HIGH",
            "名詞,一般,*,*,*,*,HIGH,*,*",
            NodeType::Unknown,
        );

        // Japanese suffix in dictionary
        check_token(
            &tokens[6],
            "帯",
            "名詞,接尾,一般,*,*,*,帯,タイ,タイ",
            NodeType::SysDict,
        );

        // Closing punctuation in dictionary
        check_token(
            &tokens[7],
            "）",
            "記号,括弧閉,*,*,*,*,）,）,）",
            NodeType::SysDict,
        );

        // Opening quote in dictionary
        check_token(
            &tokens[8],
            "「",
            "記号,括弧開,*,*,*,*,「,「,「",
            NodeType::SysDict,
        );

        // Katakana compound (brand name) - unknown
        check_token(
            &tokens[9],
            "モバキャス",
            "名詞,固有名詞,一般,*,*,*,モバキャス,*,*",
            NodeType::Unknown,
        );

        // Closing quote in dictionary
        check_token(
            &tokens[10],
            "」",
            "記号,括弧閉,*,*,*,*,」,」,」",
            NodeType::SysDict,
        );
    }

    #[test]
    fn test_tokenize_unknown_no_baseform() {
        // Equivalent to Python's TestTokenizer.test_tokenize_unknown_no_baseform()
        // Tests tokenization with baseform_unk=False - unknown words should have "*" as base_form
        let tokenizer = Tokenizer::new(None, None);
        if tokenizer.is_err() {
            eprintln!("Skipping test: SystemDictionary not available");
            return;
        }
        let tokenizer = tokenizer.unwrap();

        // Test case 1: Date text with numbers - '2009年10月16日' with baseform_unk=False
        // Numbers should be unknown with "*" as base_form, date markers should be in dictionary
        let text = "2009年10月16日";
        let results: Result<Vec<_>, _> = tokenizer.tokenize(text, None, Some(false)).collect();

        assert!(results.is_ok(), "Tokenization should succeed");
        let tokens = results.unwrap();

        assert_eq!(tokens.len(), 6, "Expected 6 tokens for '{}'", text);

        // Numbers should be unknown with "*" as base_form (baseform_unk=False)
        check_token(
            &tokens[0],
            "2009",
            "名詞,数,*,*,*,*,*,*,*",
            NodeType::Unknown,
        );

        // Date markers should be in dictionary (unchanged)
        check_token(
            &tokens[1],
            "年",
            "名詞,接尾,助数詞,*,*,*,年,ネン,ネン",
            NodeType::SysDict,
        );

        check_token(&tokens[2], "10", "名詞,数,*,*,*,*,*,*,*", NodeType::Unknown);

        check_token(
            &tokens[3],
            "月",
            "名詞,一般,*,*,*,*,月,ツキ,ツキ",
            NodeType::SysDict,
        );

        check_token(&tokens[4], "16", "名詞,数,*,*,*,*,*,*,*", NodeType::Unknown);

        check_token(
            &tokens[5],
            "日",
            "名詞,接尾,助数詞,*,*,*,日,ニチ,ニチ",
            NodeType::SysDict,
        );

        // Test case 2: Mixed Japanese/English text with baseform_unk=False
        // 'マルチメディア放送（VHF-HIGH帯）「モバキャス」'
        let text = "マルチメディア放送（VHF-HIGH帯）「モバキャス」";
        let results: Result<Vec<_>, _> = tokenizer.tokenize(text, None, Some(false)).collect();

        assert!(results.is_ok(), "Tokenization should succeed");
        let tokens = results.unwrap();

        assert_eq!(tokens.len(), 11, "Expected 11 tokens for '{}'", text);

        // Dictionary entries should be unchanged
        check_token(
            &tokens[0],
            "マルチメディア",
            "名詞,一般,*,*,*,*,マルチメディア,マルチメディア,マルチメディア",
            NodeType::SysDict,
        );

        check_token(
            &tokens[1],
            "放送",
            "名詞,サ変接続,*,*,*,*,放送,ホウソウ,ホーソー",
            NodeType::SysDict,
        );

        check_token(
            &tokens[2],
            "（",
            "記号,括弧開,*,*,*,*,（,（,（",
            NodeType::SysDict,
        );

        // Unknown words should have "*" as base_form (baseform_unk=False)
        check_token(
            &tokens[3],
            "VHF",
            "名詞,固有名詞,組織,*,*,*,*,*,*",
            NodeType::Unknown,
        );

        check_token(
            &tokens[4],
            "-",
            "名詞,サ変接続,*,*,*,*,*,*,*",
            NodeType::Unknown,
        );

        check_token(
            &tokens[5],
            "HIGH",
            "名詞,一般,*,*,*,*,*,*,*",
            NodeType::Unknown,
        );

        // Dictionary entries continue unchanged
        check_token(
            &tokens[6],
            "帯",
            "名詞,接尾,一般,*,*,*,帯,タイ,タイ",
            NodeType::SysDict,
        );

        check_token(
            &tokens[7],
            "）",
            "記号,括弧閉,*,*,*,*,）,）,）",
            NodeType::SysDict,
        );

        check_token(
            &tokens[8],
            "「",
            "記号,括弧開,*,*,*,*,「,「,「",
            NodeType::SysDict,
        );

        // Unknown compound word with "*" as base_form
        check_token(
            &tokens[9],
            "モバキャス",
            "名詞,固有名詞,一般,*,*,*,*,*,*",
            NodeType::Unknown,
        );

        check_token(
            &tokens[10],
            "」",
            "記号,括弧閉,*,*,*,*,」,」,」",
            NodeType::SysDict,
        );
    }

    #[test]
    fn test_tokenize_patched_dic() {
        // Equivalent to Python's TestTokenizer.test_tokenize_patched_dic()
        // Tests tokenization of Japanese era name "令和元年" (Reiwa Era Year 1)
        // This tests that the system dictionary includes the relatively new "令和" term
        let tokenizer = Tokenizer::new(None, None);
        if tokenizer.is_err() {
            eprintln!("Skipping test: SystemDictionary not available");
            return;
        }
        let tokenizer = tokenizer.unwrap();

        let text = "令和元年";
        let results: Result<Vec<_>, _> = tokenizer.tokenize(text, None, None).collect();

        assert!(results.is_ok(), "Tokenization should succeed");
        let tokens = results.unwrap();

        // Should produce exactly 2 tokens
        assert_eq!(tokens.len(), 2, "Expected 2 tokens for '{}'", text);

        // Token 1: 令和 (Reiwa - Japanese era name)
        check_token(
            &tokens[0],
            "令和",
            "名詞,固有名詞,一般,*,*,*,令和,レイワ,レイワ",
            NodeType::SysDict,
        );

        // Token 2: 元年 (Gannen - first year)
        check_token(
            &tokens[1],
            "元年",
            "名詞,一般,*,*,*,*,元年,ガンネン,ガンネン",
            NodeType::SysDict,
        );
    }

    #[test]
    fn test_tokenize_wakati() {
        // Equivalent to Python's TestTokenizer.test_tokenize_wakati()
        // Tests tokenization in wakati mode (分かち書き) which returns only surface forms
        // without morphological analysis information
        let tokenizer = Tokenizer::new(None, None);
        if tokenizer.is_err() {
            eprintln!("Skipping test: SystemDictionary not available");
            return;
        }
        let tokenizer = tokenizer.unwrap();

        let text = "すもももももももものうち";
        let results: Result<Vec<_>, _> = tokenizer.tokenize(text, Some(true), None).collect();

        assert!(results.is_ok(), "Tokenization should succeed");
        let tokens = results.unwrap();

        // Should produce exactly 7 tokens
        assert_eq!(tokens.len(), 7, "Expected 7 tokens for '{}'", text);

        // In wakati mode, we should get Surface results instead of Token results
        let surfaces: Vec<&str> = tokens
            .iter()
            .map(|token| match token {
                TokenizeResult::Surface(surface) => surface.as_str(),
                TokenizeResult::Token(_) => panic!("Expected Surface but got Token in wakati mode"),
            })
            .collect();

        // Validate each surface form matches expected sequence
        assert_eq!(surfaces[0], "すもも");
        assert_eq!(surfaces[1], "も");
        assert_eq!(surfaces[2], "もも");
        assert_eq!(surfaces[3], "も");
        assert_eq!(surfaces[4], "もも");
        assert_eq!(surfaces[5], "の");
        assert_eq!(surfaces[6], "うち");
    }

    #[test]
    fn test_tokenize_wakati_mode_only() {
        // Skip test if sysdic directory doesn't exist
        let sysdic_path = std::path::PathBuf::from("sysdic");
        if !sysdic_path.exists() {
            eprintln!(
                "Skipping test: sysdic directory not found at {:?}",
                sysdic_path
            );
            return;
        }

        let text = "すもももももももものうち";

        // Create tokenizer with wakati=True
        let tokenizer = Tokenizer::new(None, Some(true)).unwrap();

        // Call tokenize with wakati=False - should be ignored and return surface strings
        let results: Result<Vec<_>, _> = tokenizer.tokenize(text, Some(false), None).collect();
        assert!(results.is_ok(), "Tokenization should succeed");

        let tokens = results.unwrap();

        // Should return 7 tokens
        assert_eq!(tokens.len(), 7, "Should return 7 tokens");

        // When tokenizer is initialized with wakati=True, wakati=False parameter should be ignored
        // All tokens should be Surface variants (strings)
        for (i, token) in tokens.iter().enumerate() {
            match token {
                TokenizeResult::Surface(_) => {
                    // This is expected - wakati mode should return surface strings
                }
                TokenizeResult::Token(token) => {
                    panic!(
                        "Expected Surface but got Token '{}' at index {}",
                        token.surface(),
                        i
                    );
                }
            }
        }

        // Check specific surface forms
        let expected_surfaces = ["すもも", "も", "もも", "も", "もも", "の", "うち"];
        for (i, (token, expected)) in tokens.iter().zip(expected_surfaces.iter()).enumerate() {
            match token {
                TokenizeResult::Surface(surface) => {
                    assert_eq!(
                        surface, expected,
                        "Token {} surface mismatch: expected '{}', got '{}'",
                        i, expected, surface
                    );
                }
                TokenizeResult::Token(token) => {
                    panic!(
                        "Expected Surface but got Token '{}' at index {}",
                        token.surface(),
                        i
                    );
                }
            }
        }
    }

    #[test]
    fn test_tokenize_with_userdic() {
        // Equivalent to Python's TestTokenizer.test_tokenize_with_userdic()
        // Tests tokenization with user dictionary using IPADIC format
        use crate::dictionary::{UserDictFormat, UserDictionary};
        use std::io::Write;
        use std::sync::Arc;
        use tempfile::NamedTempFile;

        let tokenizer = Tokenizer::new(None, None);
        if tokenizer.is_err() {
            eprintln!("Skipping test: SystemDictionary not available");
            return;
        }

        // Create user dictionary CSV content (IPADIC format)
        let csv_content = "\
東京スカイツリー,1288,1288,4569,名詞,固有名詞,一般,*,*,*,東京スカイツリー,トウキョウスカイツリー,トウキョウスカイツリー
東武スカイツリーライン,1288,1288,4700,名詞,固有名詞,一般,*,*,*,東武スカイツリーライン,トウブスカイツリーライン,トウブスカイツリーライン
とうきょうスカイツリー駅,1288,1288,4143,名詞,固有名詞,一般,*,*,*,とうきょうスカイツリー駅,トウキョウスカイツリーエキ,トウキョウスカイツリーエキ";

        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        temp_file
            .write_all(csv_content.as_bytes())
            .expect("Failed to write to temp file");

        // Create system dictionary and user dictionary
        let sys_dict = crate::dictionary::SystemDictionary::instance().unwrap();
        let user_dict = UserDictionary::new(
            temp_file.path(),
            UserDictFormat::Ipadic,
            sys_dict.get_connection_matrix(),
        )
        .unwrap();

        // Create tokenizer with user dictionary
        let tokenizer = Tokenizer::with_user_dict(Arc::new(user_dict), None, None).unwrap();

        let text = "東京スカイツリーへのお越しは、東武スカイツリーライン「とうきょうスカイツリー駅」が便利です。";
        let results: Result<Vec<_>, _> = tokenizer.tokenize(text, None, None).collect();

        assert!(results.is_ok(), "Tokenization should succeed");
        let tokens = results.unwrap();

        // Should produce exactly 14 tokens (same as Python)
        assert_eq!(tokens.len(), 14, "Expected 14 tokens for '{}'", text);

        // Validate key tokens from user dictionary
        check_token(
            &tokens[0],
            "東京スカイツリー",
            "名詞,固有名詞,一般,*,*,*,東京スカイツリー,トウキョウスカイツリー,トウキョウスカイツリー",
            NodeType::UserDict,
        );
        check_token(
            &tokens[1],
            "へ",
            "助詞,格助詞,一般,*,*,*,へ,ヘ,エ",
            NodeType::SysDict,
        );
        check_token(
            &tokens[2],
            "の",
            "助詞,連体化,*,*,*,*,の,ノ,ノ",
            NodeType::SysDict,
        );
        check_token(
            &tokens[3],
            "お越し",
            "名詞,一般,*,*,*,*,お越し,オコシ,オコシ",
            NodeType::SysDict,
        );
        check_token(
            &tokens[4],
            "は",
            "助詞,係助詞,*,*,*,*,は,ハ,ワ",
            NodeType::SysDict,
        );
        check_token(
            &tokens[5],
            "、",
            "記号,読点,*,*,*,*,、,、,、",
            NodeType::SysDict,
        );
        check_token(
            &tokens[6],
            "東武スカイツリーライン",
            "名詞,固有名詞,一般,*,*,*,東武スカイツリーライン,トウブスカイツリーライン,トウブスカイツリーライン",
            NodeType::UserDict,
        );
        check_token(
            &tokens[7],
            "「",
            "記号,括弧開,*,*,*,*,「,「,「",
            NodeType::SysDict,
        );
        check_token(
            &tokens[8],
            "とうきょうスカイツリー駅",
            "名詞,固有名詞,一般,*,*,*,とうきょうスカイツリー駅,トウキョウスカイツリーエキ,トウキョウスカイツリーエキ",
            NodeType::UserDict,
        );
        check_token(
            &tokens[9],
            "」",
            "記号,括弧閉,*,*,*,*,」,」,」",
            NodeType::SysDict,
        );
        check_token(
            &tokens[10],
            "が",
            "助詞,格助詞,一般,*,*,*,が,ガ,ガ",
            NodeType::SysDict,
        );
        check_token(
            &tokens[11],
            "便利",
            "名詞,形容動詞語幹,*,*,*,*,便利,ベンリ,ベンリ",
            NodeType::SysDict,
        );
        check_token(
            &tokens[12],
            "です",
            "助動詞,*,*,*,特殊・デス,基本形,です,デス,デス",
            NodeType::SysDict,
        );
        check_token(
            &tokens[13],
            "。",
            "記号,句点,*,*,*,*,。,。,。",
            NodeType::SysDict,
        );
    }

    #[test]
    fn test_tokenize_with_simplified_userdic() {
        // Equivalent to Python's TestTokenizer.test_tokenize_with_simplified_userdic()
        // Tests tokenization with user dictionary using simplified format
        use crate::dictionary::{UserDictFormat, UserDictionary};
        use std::io::Write;
        use std::sync::Arc;
        use tempfile::NamedTempFile;

        let tokenizer = Tokenizer::new(None, None);
        if tokenizer.is_err() {
            eprintln!("Skipping test: SystemDictionary not available");
            return;
        }

        // Create user dictionary CSV content (simplified format)
        let csv_content = "\
東京スカイツリー,カスタム名詞,トウキョウスカイツリー
東武スカイツリーライン,カスタム名詞,トウブスカイツリーライン
とうきょうスカイツリー駅,カスタム名詞,トウキョウスカイツリーエキ";

        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        temp_file
            .write_all(csv_content.as_bytes())
            .expect("Failed to write to temp file");

        // Create system dictionary and user dictionary
        let sys_dict = crate::dictionary::SystemDictionary::instance().unwrap();
        let user_dict = UserDictionary::new(
            temp_file.path(),
            UserDictFormat::Simpledic,
            sys_dict.get_connection_matrix(),
        )
        .unwrap();

        // Create tokenizer with user dictionary
        let tokenizer = Tokenizer::with_user_dict(Arc::new(user_dict), None, None).unwrap();

        let text = "東京スカイツリーへのお越しは、東武スカイツリーライン「とうきょうスカイツリー駅」が便利です。";
        let results: Result<Vec<_>, _> = tokenizer.tokenize(text, None, None).collect();

        if let Err(ref e) = results {
            eprintln!("Tokenization error: {:?}", e);
        }
        assert!(results.is_ok(), "Tokenization should succeed");
        let tokens = results.unwrap();

        // Should produce exactly 14 tokens (same as Python)
        assert_eq!(tokens.len(), 14, "Expected 14 tokens for '{}'", text);

        // Validate key tokens from user dictionary (simplified format)
        check_token(
            &tokens[0],
            "東京スカイツリー",
            "カスタム名詞,*,*,*,*,*,東京スカイツリー,トウキョウスカイツリー,トウキョウスカイツリー",
            NodeType::UserDict,
        );
        check_token(
            &tokens[1],
            "へ",
            "助詞,格助詞,一般,*,*,*,へ,ヘ,エ",
            NodeType::SysDict,
        );
        check_token(
            &tokens[2],
            "の",
            "助詞,連体化,*,*,*,*,の,ノ,ノ",
            NodeType::SysDict,
        );
        check_token(
            &tokens[3],
            "お越し",
            "名詞,一般,*,*,*,*,お越し,オコシ,オコシ",
            NodeType::SysDict,
        );
        check_token(
            &tokens[4],
            "は",
            "助詞,係助詞,*,*,*,*,は,ハ,ワ",
            NodeType::SysDict,
        );
        check_token(
            &tokens[5],
            "、",
            "記号,読点,*,*,*,*,、,、,、",
            NodeType::SysDict,
        );
        check_token(
            &tokens[6],
            "東武スカイツリーライン",
            "カスタム名詞,*,*,*,*,*,東武スカイツリーライン,トウブスカイツリーライン,トウブスカイツリーライン",
            NodeType::UserDict,
        );
        check_token(
            &tokens[7],
            "「",
            "記号,括弧開,*,*,*,*,「,「,「",
            NodeType::SysDict,
        );
        check_token(
            &tokens[8],
            "とうきょうスカイツリー駅",
            "カスタム名詞,*,*,*,*,*,とうきょうスカイツリー駅,トウキョウスカイツリーエキ,トウキョウスカイツリーエキ",
            NodeType::UserDict,
        );
        check_token(
            &tokens[9],
            "」",
            "記号,括弧閉,*,*,*,*,」,」,」",
            NodeType::SysDict,
        );
        check_token(
            &tokens[10],
            "が",
            "助詞,格助詞,一般,*,*,*,が,ガ,ガ",
            NodeType::SysDict,
        );
        check_token(
            &tokens[11],
            "便利",
            "名詞,形容動詞語幹,*,*,*,*,便利,ベンリ,ベンリ",
            NodeType::SysDict,
        );
        check_token(
            &tokens[12],
            "です",
            "助動詞,*,*,*,特殊・デス,基本形,です,デス,デス",
            NodeType::SysDict,
        );
        check_token(
            &tokens[13],
            "。",
            "記号,句点,*,*,*,*,。,。,。",
            NodeType::SysDict,
        );
    }

    #[test]
    fn test_tokenize_with_userdic_wakati() {
        // Equivalent to Python's TestTokenizer.test_tokenize_with_userdic_wakati()
        // Tests tokenization with user dictionary in wakati mode
        use crate::dictionary::{UserDictFormat, UserDictionary};
        use std::io::Write;
        use std::sync::Arc;
        use tempfile::NamedTempFile;

        let tokenizer = Tokenizer::new(None, None);
        if tokenizer.is_err() {
            eprintln!("Skipping test: SystemDictionary not available");
            return;
        }

        // Create user dictionary CSV content (IPADIC format)
        let csv_content = "\
東京スカイツリー,1288,1288,4569,名詞,固有名詞,一般,*,*,*,東京スカイツリー,トウキョウスカイツリー,トウキョウスカイツリー
東武スカイツリーライン,1288,1288,4700,名詞,固有名詞,一般,*,*,*,東武スカイツリーライン,トウブスカイツリーライン,トウブスカイツリーライン
とうきょうスカイツリー駅,1288,1288,4143,名詞,固有名詞,一般,*,*,*,とうきょうスカイツリー駅,トウキョウスカイツリーエキ,トウキョウスカイツリーエキ";

        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        temp_file
            .write_all(csv_content.as_bytes())
            .expect("Failed to write to temp file");

        // Create system dictionary and user dictionary
        let sys_dict = crate::dictionary::SystemDictionary::instance().unwrap();
        let user_dict = UserDictionary::new(
            temp_file.path(),
            UserDictFormat::Ipadic,
            sys_dict.get_connection_matrix(),
        )
        .unwrap();

        // Create tokenizer with user dictionary in wakati mode
        let tokenizer = Tokenizer::with_user_dict(Arc::new(user_dict), None, Some(true)).unwrap();

        let text = "東京スカイツリーへのお越しは、東武スカイツリーライン「とうきょうスカイツリー駅」が便利です。";
        let results: Result<Vec<_>, _> = tokenizer.tokenize(text, Some(true), None).collect();

        assert!(results.is_ok(), "Tokenization should succeed");
        let tokens = results.unwrap();

        // Should produce exactly 14 tokens (same as Python)
        assert_eq!(tokens.len(), 14, "Expected 14 tokens for '{}'", text);

        // In wakati mode, we should get Surface results instead of Token results
        let surfaces: Vec<&str> = tokens
            .iter()
            .map(|token| match token {
                TokenizeResult::Surface(surface) => surface.as_str(),
                TokenizeResult::Token(_) => panic!("Expected Surface but got Token in wakati mode"),
            })
            .collect();

        // Validate each surface form matches expected sequence
        assert_eq!(surfaces[0], "東京スカイツリー");
        assert_eq!(surfaces[1], "へ");
        assert_eq!(surfaces[2], "の");
        assert_eq!(surfaces[3], "お越し");
        assert_eq!(surfaces[4], "は");
        assert_eq!(surfaces[5], "、");
        assert_eq!(surfaces[6], "東武スカイツリーライン");
        assert_eq!(surfaces[7], "「");
        assert_eq!(surfaces[8], "とうきょうスカイツリー駅");
        assert_eq!(surfaces[9], "」");
        assert_eq!(surfaces[10], "が");
        assert_eq!(surfaces[11], "便利");
        assert_eq!(surfaces[12], "です");
        assert_eq!(surfaces[13], "。");
    }

    #[test]
    fn test_tokenize_large_text() {
        // Equivalent to Python's TestTokenizer.test_tokenize_large_text()
        // Tests tokenization of a large text file to ensure the tokenizer can handle
        // substantial amounts of text without errors
        use std::fs;

        let tokenizer = Tokenizer::new(None, None);
        if tokenizer.is_err() {
            eprintln!("Skipping test: SystemDictionary not available");
            return;
        }
        let tokenizer = tokenizer.unwrap();

        // Read the large text file
        let text = match fs::read_to_string("fixtures/cases/text_lemon.txt") {
            Ok(content) => content,
            Err(e) => {
                eprintln!("Skipping test: Could not read fixtures/cases/text_lemon.txt: {}", e);
                return;
            }
        };

        // Tokenize the text - this should complete without errors
        let results: Result<Vec<_>, _> = tokenizer.tokenize(&text, None, None).collect();

        assert!(results.is_ok(), "Tokenization of large text should succeed");
        let tokens = results.unwrap();

        // Ensure we get a reasonable number of tokens (the exact count may vary
        // but it should be substantial for a large text)
        assert!(
            tokens.len() > 100,
            "Large text should produce many tokens, got {}",
            tokens.len()
        );
    }
}
