use std::io::Write;
use std::path::Path;

use crate::dictionary::{ConnectionMatrix, DictEntry, UserDictFormat, UserDictionary};
use crate::error::RunomeError;

/// Helper function to create a temporary CSV file with given content
fn create_temp_csv(content: &str) -> tempfile::NamedTempFile {
    let mut temp_file = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    temp_file
        .write_all(content.as_bytes())
        .expect("Failed to write to temp file");
    temp_file
}

/// Helper function to create mock connection matrix
fn create_mock_connections() -> ConnectionMatrix {
    // Create a simple 3x3 connection matrix for testing
    ConnectionMatrix::from_rows(&[vec![0, 1, 2], vec![1, 0, 3], vec![2, 3, 0]])
        .expect("Failed to build mock connection matrix")
}

#[cfg(test)]
mod csv_parsing_tests {
    use super::*;

    #[test]
    fn test_parse_ipadic_line_valid() {
        let line = "東京スカイツリー,1288,1288,4569,名詞,固有名詞,一般,*,*,*,東京スカイツリー,トウキョウスカイツリー,トウキョウスカイツリー";
        let entry = UserDictionary::parse_ipadic_line(line, 0).unwrap();

        assert_eq!(entry.surface, "東京スカイツリー");
        assert_eq!(entry.left_id, 1288);
        assert_eq!(entry.right_id, 1288);
        assert_eq!(entry.cost, 4569);
        assert_eq!(entry.part_of_speech, "名詞,固有名詞,一般,*");
        assert_eq!(entry.inflection_type, "*");
        assert_eq!(entry.inflection_form, "*");
        assert_eq!(entry.base_form, "東京スカイツリー");
        assert_eq!(entry.reading, "トウキョウスカイツリー");
        assert_eq!(entry.phonetic, "トウキョウスカイツリー");
        assert_eq!(entry.morph_id, 0);
    }

    #[test]
    fn test_parse_ipadic_line_invalid_field_count() {
        let line = "東京スカイツリー,1288,1288,4569,名詞,固有名詞"; // Only 6 fields
        let result = UserDictionary::parse_ipadic_line(line, 0);

        assert!(result.is_err());
        match result.unwrap_err() {
            RunomeError::CsvParseError { line, reason } => {
                assert_eq!(line, 1);
                assert!(reason.contains("Expected 13 fields, got 6"));
            }
            _ => panic!("Expected CsvParseError"),
        }
    }

    #[test]
    fn test_parse_ipadic_line_invalid_numeric_fields() {
        let line = "東京スカイツリー,invalid,1288,4569,名詞,固有名詞,一般,*,*,*,東京スカイツリー,トウキョウスカイツリー,トウキョウスカイツリー";
        let result = UserDictionary::parse_ipadic_line(line, 0);

        assert!(result.is_err());
        match result.unwrap_err() {
            RunomeError::CsvParseError { line, reason } => {
                assert_eq!(line, 1);
                assert!(reason.contains("Failed to parse left_id"));
            }
            _ => panic!("Expected CsvParseError"),
        }
    }

    #[test]
    fn test_parse_simpledic_line_valid() {
        let line = "東京スカイツリー,カスタム名詞,トウキョウスカイツリー";
        let entry = UserDictionary::parse_simpledic_line(line, 0).unwrap();

        assert_eq!(entry.surface, "東京スカイツリー");
        assert_eq!(entry.left_id, 0);
        assert_eq!(entry.right_id, 0);
        assert_eq!(entry.cost, -32000);
        assert_eq!(entry.part_of_speech, "カスタム名詞,*,*,*");
        assert_eq!(entry.inflection_type, "*");
        assert_eq!(entry.inflection_form, "*");
        assert_eq!(entry.base_form, "東京スカイツリー");
        assert_eq!(entry.reading, "トウキョウスカイツリー");
        assert_eq!(entry.phonetic, "トウキョウスカイツリー");
        assert_eq!(entry.morph_id, 0);
    }

    #[test]
    fn test_parse_simpledic_line_invalid_field_count() {
        let line = "東京スカイツリー,カスタム名詞"; // Only 2 fields
        let result = UserDictionary::parse_simpledic_line(line, 0);

        assert!(result.is_err());
        match result.unwrap_err() {
            RunomeError::CsvParseError { line, reason } => {
                assert_eq!(line, 1);
                assert!(reason.contains("Expected 3 fields, got 2"));
            }
            _ => panic!("Expected CsvParseError"),
        }
    }

    #[test]
    fn test_load_entries_ipadic_format() {
        let csv_content = "\
東京スカイツリー,1288,1288,4569,名詞,固有名詞,一般,*,*,*,東京スカイツリー,トウキョウスカイツリー,トウキョウスカイツリー
東武スカイツリーライン,1288,1288,4700,名詞,固有名詞,一般,*,*,*,東武スカイツリーライン,トウブスカイツリーライン,トウブスカイツリーライン
とうきょうスカイツリー駅,1288,1288,4143,名詞,固有名詞,一般,*,*,*,とうきょうスカイツリー駅,トウキョウスカイツリーエキ,トウキョウスカイツリーエキ";

        let temp_file = create_temp_csv(csv_content);
        let entries =
            UserDictionary::load_entries(temp_file.path(), UserDictFormat::Ipadic).unwrap();

        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].surface, "東京スカイツリー");
        assert_eq!(entries[1].surface, "東武スカイツリーライン");
        assert_eq!(entries[2].surface, "とうきょうスカイツリー駅");

        // Check morph_id is set correctly
        assert_eq!(entries[0].morph_id, 0);
        assert_eq!(entries[1].morph_id, 1);
        assert_eq!(entries[2].morph_id, 2);
    }

    #[test]
    fn test_load_entries_simpledic_format() {
        let csv_content = "\
東京スカイツリー,カスタム名詞,トウキョウスカイツリー
東武スカイツリーライン,カスタム名詞,トウブスカイツリーライン
とうきょうスカイツリー駅,カスタム名詞,トウキョウスカイツリーエキ";

        let temp_file = create_temp_csv(csv_content);
        let entries =
            UserDictionary::load_entries(temp_file.path(), UserDictFormat::Simpledic).unwrap();

        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].surface, "東京スカイツリー");
        assert_eq!(entries[0].part_of_speech, "カスタム名詞,*,*,*");
        assert_eq!(entries[0].cost, -32000);
        assert_eq!(entries[0].left_id, 0);
        assert_eq!(entries[0].right_id, 0);
    }

    #[test]
    fn test_load_entries_with_empty_lines() {
        let csv_content = "\
東京スカイツリー,1288,1288,4569,名詞,固有名詞,一般,*,*,*,東京スカイツリー,トウキョウスカイツリー,トウキョウスカイツリー

東武スカイツリーライン,1288,1288,4700,名詞,固有名詞,一般,*,*,*,東武スカイツリーライン,トウブスカイツリーライン,トウブスカイツリーライン
    
とうきょうスカイツリー駅,1288,1288,4143,名詞,固有名詞,一般,*,*,*,とうきょうスカイツリー駅,トウキョウスカイツリーエキ,トウキョウスカイツリーエキ";

        let temp_file = create_temp_csv(csv_content);
        let entries =
            UserDictionary::load_entries(temp_file.path(), UserDictFormat::Ipadic).unwrap();

        // Should skip empty lines
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].surface, "東京スカイツリー");
        assert_eq!(entries[1].surface, "東武スカイツリーライン");
        assert_eq!(entries[2].surface, "とうきょうスカイツリー駅");
    }

    #[test]
    fn test_load_entries_empty_file() {
        let csv_content = "";

        let temp_file = create_temp_csv(csv_content);
        let result = UserDictionary::load_entries(temp_file.path(), UserDictFormat::Ipadic);

        assert!(result.is_err());
        match result.unwrap_err() {
            RunomeError::UserDictError { reason } => {
                assert!(reason.contains("No valid entries found"));
            }
            _ => panic!("Expected UserDictError"),
        }
    }

    #[test]
    fn test_load_entries_multiple_entries_same_surface() {
        let csv_content = "\
東京,1288,1288,4569,名詞,固有名詞,地域,一般,*,*,東京,トウキョウ,トウキョウ
東京,1285,1285,4000,名詞,固有名詞,人名,一般,*,*,東京,トウキョウ,トウキョウ";

        let temp_file = create_temp_csv(csv_content);
        let entries =
            UserDictionary::load_entries(temp_file.path(), UserDictFormat::Ipadic).unwrap();

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].surface, "東京");
        assert_eq!(entries[1].surface, "東京");
        assert_eq!(entries[0].part_of_speech, "名詞,固有名詞,地域,一般");
        assert_eq!(entries[1].part_of_speech, "名詞,固有名詞,人名,一般");
        assert_eq!(entries[0].morph_id, 0);
        assert_eq!(entries[1].morph_id, 1);
    }

    #[test]
    fn test_load_entries_nonexistent_file() {
        let nonexistent_path = Path::new("/nonexistent/file.csv");
        let result = UserDictionary::load_entries(nonexistent_path, UserDictFormat::Ipadic);

        assert!(result.is_err());
        match result.unwrap_err() {
            RunomeError::UserDictError { reason } => {
                assert!(reason.contains("Failed to read CSV file"));
            }
            _ => panic!("Expected UserDictError"),
        }
    }
}

#[cfg(test)]
mod fst_building_tests {
    use super::*;

    #[test]
    fn test_build_fst_single_entry() {
        let entries = vec![DictEntry {
            surface: "東京".to_string(),
            left_id: 1288,
            right_id: 1288,
            cost: 4569,
            part_of_speech: "名詞,固有名詞,一般,*".to_string(),
            inflection_type: "*".to_string(),
            inflection_form: "*".to_string(),
            base_form: "東京".to_string(),
            reading: "トウキョウ".to_string(),
            phonetic: "トウキョウ".to_string(),
            morph_id: 0,
        }];

        let (matcher, morpheme_index) = UserDictionary::build_fst(&entries).unwrap();

        // Should have one entry in morpheme index
        assert_eq!(morpheme_index.len(), 1);
        assert_eq!(morpheme_index[0], vec![0]);

        // Test FST matching
        let (matched, index_ids) = matcher.run("東京", true).unwrap();
        assert!(matched);
        assert_eq!(index_ids.len(), 1);
        assert_eq!(index_ids[0], 0);
    }

    #[test]
    fn test_build_fst_multiple_entries_same_surface() {
        let entries = vec![
            DictEntry {
                surface: "東京".to_string(),
                left_id: 1288,
                right_id: 1288,
                cost: 4569,
                part_of_speech: "名詞,固有名詞,地域,一般".to_string(),
                inflection_type: "*".to_string(),
                inflection_form: "*".to_string(),
                base_form: "東京".to_string(),
                reading: "トウキョウ".to_string(),
                phonetic: "トウキョウ".to_string(),
                morph_id: 0,
            },
            DictEntry {
                surface: "東京".to_string(),
                left_id: 1285,
                right_id: 1285,
                cost: 4000,
                part_of_speech: "名詞,固有名詞,人名,一般".to_string(),
                inflection_type: "*".to_string(),
                inflection_form: "*".to_string(),
                base_form: "東京".to_string(),
                reading: "トウキョウ".to_string(),
                phonetic: "トウキョウ".to_string(),
                morph_id: 1,
            },
        ];

        let (matcher, morpheme_index) = UserDictionary::build_fst(&entries).unwrap();

        // Should have one entry in morpheme index containing both morpheme IDs
        assert_eq!(morpheme_index.len(), 1);
        assert_eq!(morpheme_index[0], vec![0, 1]);

        // Test FST matching
        let (matched, index_ids) = matcher.run("東京", true).unwrap();
        assert!(matched);
        assert_eq!(index_ids.len(), 1);
        assert_eq!(index_ids[0], 0);
    }

    #[test]
    fn test_build_fst_multiple_different_surfaces() {
        let entries = vec![
            DictEntry {
                surface: "東京".to_string(),
                left_id: 1288,
                right_id: 1288,
                cost: 4569,
                part_of_speech: "名詞,固有名詞,一般,*".to_string(),
                inflection_type: "*".to_string(),
                inflection_form: "*".to_string(),
                base_form: "東京".to_string(),
                reading: "トウキョウ".to_string(),
                phonetic: "トウキョウ".to_string(),
                morph_id: 0,
            },
            DictEntry {
                surface: "大阪".to_string(),
                left_id: 1288,
                right_id: 1288,
                cost: 4000,
                part_of_speech: "名詞,固有名詞,一般,*".to_string(),
                inflection_type: "*".to_string(),
                inflection_form: "*".to_string(),
                base_form: "大阪".to_string(),
                reading: "オオサカ".to_string(),
                phonetic: "オーサカ".to_string(),
                morph_id: 1,
            },
        ];

        let (matcher, morpheme_index) = UserDictionary::build_fst(&entries).unwrap();

        // Should have two entries in morpheme index
        assert_eq!(morpheme_index.len(), 2);

        // Test FST matching for both entries
        let (matched1, index_ids1) = matcher.run("東京", true).unwrap();
        assert!(matched1);
        assert_eq!(index_ids1.len(), 1);

        let (matched2, index_ids2) = matcher.run("大阪", true).unwrap();
        assert!(matched2);
        assert_eq!(index_ids2.len(), 1);

        // Should not match non-existent surface
        let (matched3, index_ids3) = matcher.run("名古屋", true).unwrap();
        assert!(!matched3);
        assert_eq!(index_ids3.len(), 0);
    }

    #[test]
    fn test_build_fst_empty_entries() {
        let entries = vec![];
        let (matcher, morpheme_index) = UserDictionary::build_fst(&entries).unwrap();

        assert_eq!(morpheme_index.len(), 0);

        // Should not match anything
        let (matched, index_ids) = matcher.run("東京", true).unwrap();
        assert!(!matched);
        assert_eq!(index_ids.len(), 0);
    }

    #[test]
    fn test_build_fst_complex_multiple_entries() {
        // Test with multiple entries having the same surface form and different ones
        let entries = vec![
            DictEntry {
                surface: "東京".to_string(),
                left_id: 1288,
                right_id: 1288,
                cost: 4569,
                part_of_speech: "名詞,固有名詞,地域,一般".to_string(),
                inflection_type: "*".to_string(),
                inflection_form: "*".to_string(),
                base_form: "東京".to_string(),
                reading: "トウキョウ".to_string(),
                phonetic: "トウキョウ".to_string(),
                morph_id: 0,
            },
            DictEntry {
                surface: "東京".to_string(),
                left_id: 1285,
                right_id: 1285,
                cost: 4000,
                part_of_speech: "名詞,固有名詞,人名,一般".to_string(),
                inflection_type: "*".to_string(),
                inflection_form: "*".to_string(),
                base_form: "東京".to_string(),
                reading: "トウキョウ".to_string(),
                phonetic: "トウキョウ".to_string(),
                morph_id: 1,
            },
            DictEntry {
                surface: "東京".to_string(),
                left_id: 1290,
                right_id: 1290,
                cost: 3500,
                part_of_speech: "名詞,固有名詞,組織,一般".to_string(),
                inflection_type: "*".to_string(),
                inflection_form: "*".to_string(),
                base_form: "東京".to_string(),
                reading: "トウキョウ".to_string(),
                phonetic: "トウキョウ".to_string(),
                morph_id: 2,
            },
            DictEntry {
                surface: "大阪".to_string(),
                left_id: 1288,
                right_id: 1288,
                cost: 4200,
                part_of_speech: "名詞,固有名詞,地域,一般".to_string(),
                inflection_type: "*".to_string(),
                inflection_form: "*".to_string(),
                base_form: "大阪".to_string(),
                reading: "オオサカ".to_string(),
                phonetic: "オーサカ".to_string(),
                morph_id: 3,
            },
        ];

        let (matcher, morpheme_index) = UserDictionary::build_fst(&entries).unwrap();

        // Should have two entries in morpheme index (one for each unique surface)
        assert_eq!(morpheme_index.len(), 2);

        // Test FST matching for "東京" - should get 3 morpheme IDs
        let (matched1, index_ids1) = matcher.run("東京", true).unwrap();
        assert!(matched1);
        assert_eq!(index_ids1.len(), 1);

        let morpheme_ids_tokyo = &morpheme_index[index_ids1[0] as usize];
        assert_eq!(morpheme_ids_tokyo.len(), 3);
        assert!(morpheme_ids_tokyo.contains(&0));
        assert!(morpheme_ids_tokyo.contains(&1));
        assert!(morpheme_ids_tokyo.contains(&2));

        // Test FST matching for "大阪" - should get 1 morpheme ID
        let (matched2, index_ids2) = matcher.run("大阪", true).unwrap();
        assert!(matched2);
        assert_eq!(index_ids2.len(), 1);

        let morpheme_ids_osaka = &morpheme_index[index_ids2[0] as usize];
        assert_eq!(morpheme_ids_osaka.len(), 1);
        assert!(morpheme_ids_osaka.contains(&3));
    }

    #[test]
    fn test_build_fst_utf8_surfaces() {
        // Test with various UTF-8 characters
        let entries = vec![
            DictEntry {
                surface: "こんにちは".to_string(), // Hiragana
                left_id: 1,
                right_id: 1,
                cost: 100,
                part_of_speech: "感動詞,*,*,*".to_string(),
                inflection_type: "*".to_string(),
                inflection_form: "*".to_string(),
                base_form: "こんにちは".to_string(),
                reading: "コンニチワ".to_string(),
                phonetic: "コンニチワ".to_string(),
                morph_id: 0,
            },
            DictEntry {
                surface: "カタカナ".to_string(), // Katakana
                left_id: 2,
                right_id: 2,
                cost: 200,
                part_of_speech: "名詞,一般,*,*".to_string(),
                inflection_type: "*".to_string(),
                inflection_form: "*".to_string(),
                base_form: "カタカナ".to_string(),
                reading: "カタカナ".to_string(),
                phonetic: "カタカナ".to_string(),
                morph_id: 1,
            },
            DictEntry {
                surface: "漢字".to_string(), // Kanji
                left_id: 3,
                right_id: 3,
                cost: 300,
                part_of_speech: "名詞,一般,*,*".to_string(),
                inflection_type: "*".to_string(),
                inflection_form: "*".to_string(),
                base_form: "漢字".to_string(),
                reading: "カンジ".to_string(),
                phonetic: "カンジ".to_string(),
                morph_id: 2,
            },
        ];

        let (matcher, morpheme_index) = UserDictionary::build_fst(&entries).unwrap();

        // Should have three entries in morpheme index
        assert_eq!(morpheme_index.len(), 3);

        // Test each UTF-8 surface form
        let test_cases = ["こんにちは", "カタカナ", "漢字"];
        for surface in test_cases {
            let (matched, index_ids) = matcher.run(surface, true).unwrap();
            assert!(matched, "Should match surface: {}", surface);
            assert_eq!(
                index_ids.len(),
                1,
                "Should have one index ID for: {}",
                surface
            );
        }
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_user_dictionary_creation_ipadic() {
        let csv_content = "\
東京スカイツリー,1288,1288,4569,名詞,固有名詞,一般,*,*,*,東京スカイツリー,トウキョウスカイツリー,トウキョウスカイツリー
東武スカイツリーライン,1288,1288,4700,名詞,固有名詞,一般,*,*,*,東武スカイツリーライン,トウブスカイツリーライン,トウブスカイツリーライン";

        let temp_file = create_temp_csv(csv_content);
        let connections = create_mock_connections();

        let user_dict = UserDictionary::new(temp_file.path(), UserDictFormat::Ipadic, connections);
        assert!(user_dict.is_ok());

        let user_dict = user_dict.unwrap();
        assert_eq!(user_dict.entries.len(), 2);
    }

    #[test]
    fn test_user_dictionary_creation_simpledic() {
        let csv_content = "\
東京スカイツリー,カスタム名詞,トウキョウスカイツリー
東武スカイツリーライン,カスタム名詞,トウブスカイツリーライン";

        let temp_file = create_temp_csv(csv_content);
        let connections = create_mock_connections();

        let user_dict =
            UserDictionary::new(temp_file.path(), UserDictFormat::Simpledic, connections);
        assert!(user_dict.is_ok());

        let user_dict = user_dict.unwrap();
        assert_eq!(user_dict.entries.len(), 2);
    }

    #[test]
    fn test_user_dictionary_creation_invalid_file() {
        let nonexistent_path = Path::new("/nonexistent/file.csv");
        let connections = create_mock_connections();

        let result = UserDictionary::new(nonexistent_path, UserDictFormat::Ipadic, connections);
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod edge_case_tests {
    use super::*;
    use crate::dictionary::Dictionary;

    #[test]
    fn test_lookup_with_special_characters() {
        let csv_content = "\
@,1,1,100,記号,一般,*,*,*,*,@,アットマーク,アットマーク
#,2,2,200,記号,一般,*,*,*,*,#,ハッシュ,ハッシュ
$,3,3,300,記号,一般,*,*,*,*,$,ドル,ドル
%,4,4,400,記号,一般,*,*,*,*,%,パーセント,パーセント";

        let temp_file = create_temp_csv(csv_content);
        let connections = create_mock_connections();

        let user_dict =
            UserDictionary::new(temp_file.path(), UserDictFormat::Ipadic, connections).unwrap();

        // Test lookup for special characters
        let test_cases = [
            ("@", "アットマーク"),
            ("#", "ハッシュ"),
            ("$", "ドル"),
            ("%", "パーセント"),
        ];

        for (surface, expected_reading) in test_cases {
            let results = user_dict.lookup(surface).unwrap();
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].surface, surface);
            assert_eq!(results[0].reading, expected_reading);
        }
    }

    #[test]
    fn test_lookup_with_very_long_surface() {
        let long_surface = "あ".repeat(100);
        let csv_content = format!(
            "{},1,1,100,名詞,一般,*,*,*,*,{},アア,アア",
            long_surface, long_surface
        );

        let temp_file = create_temp_csv(&csv_content);
        let connections = create_mock_connections();

        let user_dict =
            UserDictionary::new(temp_file.path(), UserDictFormat::Ipadic, connections).unwrap();

        // Test lookup for very long surface form
        let results = user_dict.lookup(&long_surface).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].surface, long_surface);
    }

    #[test]
    fn test_lookup_with_mixed_scripts() {
        let csv_content = "\
hello世界,1,1,100,名詞,一般,*,*,*,*,hello世界,ハローセカイ,ハローセカイ
123あいう,2,2,200,名詞,一般,*,*,*,*,123あいう,イチニサンアイウ,イチニサンアイウ
アイウ456,3,3,300,名詞,一般,*,*,*,*,アイウ456,アイウヨンゴロク,アイウヨンゴロク";

        let temp_file = create_temp_csv(csv_content);
        let connections = create_mock_connections();

        let user_dict =
            UserDictionary::new(temp_file.path(), UserDictFormat::Ipadic, connections).unwrap();

        // Test lookup for mixed script entries
        let test_cases = [
            ("hello世界", "ハローセカイ"),
            ("123あいう", "イチニサンアイウ"),
            ("アイウ456", "アイウヨンゴロク"),
        ];

        for (surface, expected_reading) in test_cases {
            let results = user_dict.lookup(surface).unwrap();
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].surface, surface);
            assert_eq!(results[0].reading, expected_reading);
        }
    }

    #[test]
    fn test_lookup_with_extreme_costs() {
        let csv_content = "\
最小コスト,1,1,-32767,名詞,一般,*,*,*,*,最小コスト,サイショウコスト,サイショウコスト
最大コスト,2,2,32767,名詞,一般,*,*,*,*,最大コスト,サイダイコスト,サイダイコスト
ゼロコスト,3,3,0,名詞,一般,*,*,*,*,ゼロコスト,ゼロコスト,ゼロコスト";

        let temp_file = create_temp_csv(csv_content);
        let connections = create_mock_connections();

        let user_dict =
            UserDictionary::new(temp_file.path(), UserDictFormat::Ipadic, connections).unwrap();

        // Test lookup for entries with extreme costs
        let results_min = user_dict.lookup("最小コスト").unwrap();
        assert_eq!(results_min.len(), 1);
        assert_eq!(results_min[0].cost, -32767);

        let results_max = user_dict.lookup("最大コスト").unwrap();
        assert_eq!(results_max.len(), 1);
        assert_eq!(results_max[0].cost, 32767);

        let results_zero = user_dict.lookup("ゼロコスト").unwrap();
        assert_eq!(results_zero.len(), 1);
        assert_eq!(results_zero[0].cost, 0);
    }

    #[test]
    fn test_lookup_morpheme_ids_out_of_bounds() {
        let csv_content = "\
テスト,1,1,100,名詞,一般,*,*,*,*,テスト,テスト,テスト";

        let temp_file = create_temp_csv(csv_content);
        let connections = create_mock_connections();

        let user_dict =
            UserDictionary::new(temp_file.path(), UserDictFormat::Ipadic, connections).unwrap();

        // Test lookup_morpheme_ids with out-of-bounds index
        let morpheme_ids = user_dict.lookup_morpheme_ids(999);
        assert_eq!(morpheme_ids.len(), 0);
    }

    #[test]
    fn test_dictionary_trait_consistency() {
        let csv_content = "\
テスト,1,1,100,名詞,一般,*,*,*,*,テスト,テスト,テスト";

        let temp_file = create_temp_csv(csv_content);
        let connections = create_mock_connections();

        let user_dict =
            UserDictionary::new(temp_file.path(), UserDictFormat::Ipadic, connections).unwrap();

        // Test that the same lookup returns identical results
        let results1 = user_dict.lookup("テスト").unwrap();
        let results2 = user_dict.lookup("テスト").unwrap();

        assert_eq!(results1.len(), results2.len());
        for (r1, r2) in results1.iter().zip(results2.iter()) {
            assert_eq!(r1.surface, r2.surface);
            assert_eq!(r1.left_id, r2.left_id);
            assert_eq!(r1.right_id, r2.right_id);
            assert_eq!(r1.cost, r2.cost);
            assert_eq!(r1.part_of_speech, r2.part_of_speech);
            assert_eq!(r1.reading, r2.reading);
            assert_eq!(r1.phonetic, r2.phonetic);
            assert_eq!(r1.morph_id, r2.morph_id);
        }
    }
}

#[cfg(test)]
mod dictionary_lookup_tests {
    use super::*;
    use crate::dictionary::Dictionary;

    #[test]
    fn test_dictionary_lookup_single_entry() {
        let csv_content = "\
東京スカイツリー,1288,1288,4569,名詞,固有名詞,一般,*,*,*,東京スカイツリー,トウキョウスカイツリー,トウキョウスカイツリー";

        let temp_file = create_temp_csv(csv_content);
        let connections = create_mock_connections();

        let user_dict =
            UserDictionary::new(temp_file.path(), UserDictFormat::Ipadic, connections).unwrap();

        // Test successful lookup
        let results = user_dict.lookup("東京スカイツリー").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].surface, "東京スカイツリー");
        assert_eq!(results[0].left_id, 1288);
        assert_eq!(results[0].right_id, 1288);
        assert_eq!(results[0].cost, 4569);
        assert_eq!(results[0].part_of_speech, "名詞,固有名詞,一般,*");
        assert_eq!(results[0].reading, "トウキョウスカイツリー");
        assert_eq!(results[0].phonetic, "トウキョウスカイツリー");
        assert_eq!(results[0].morph_id, 0);

        // Test non-existent entry
        let results = user_dict.lookup("存在しない").unwrap();
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_dictionary_lookup_multiple_entries_same_surface() {
        let csv_content = "\
東京,1288,1288,4569,名詞,固有名詞,地域,一般,*,*,東京,トウキョウ,トウキョウ
東京,1285,1285,4000,名詞,固有名詞,人名,一般,*,*,東京,トウキョウ,トウキョウ
東京,1290,1290,3500,名詞,固有名詞,組織,一般,*,*,東京,トウキョウ,トウキョウ";

        let temp_file = create_temp_csv(csv_content);
        let connections = create_mock_connections();

        let user_dict =
            UserDictionary::new(temp_file.path(), UserDictFormat::Ipadic, connections).unwrap();

        // Test lookup returns all entries for the same surface
        let results = user_dict.lookup("東京").unwrap();
        assert_eq!(results.len(), 3);

        // Check that all entries have the same surface but different properties
        for result in &results {
            assert_eq!(result.surface, "東京");
            assert_eq!(result.reading, "トウキョウ");
            assert_eq!(result.phonetic, "トウキョウ");
        }

        // Check that we have the different part-of-speech entries
        let pos_set: std::collections::HashSet<_> =
            results.iter().map(|r| r.part_of_speech.clone()).collect();
        assert!(pos_set.contains("名詞,固有名詞,地域,一般"));
        assert!(pos_set.contains("名詞,固有名詞,人名,一般"));
        assert!(pos_set.contains("名詞,固有名詞,組織,一般"));

        // Check that we have different costs
        let costs: Vec<_> = results.iter().map(|r| r.cost).collect();
        assert!(costs.contains(&4569));
        assert!(costs.contains(&4000));
        assert!(costs.contains(&3500));
    }

    #[test]
    fn test_dictionary_lookup_mixed_entries() {
        let csv_content = "\
東京,1288,1288,4569,名詞,固有名詞,地域,一般,*,*,東京,トウキョウ,トウキョウ
大阪,1288,1288,4200,名詞,固有名詞,地域,一般,*,*,大阪,オオサカ,オーサカ
東京,1285,1285,4000,名詞,固有名詞,人名,一般,*,*,東京,トウキョウ,トウキョウ
名古屋,1288,1288,4300,名詞,固有名詞,地域,一般,*,*,名古屋,ナゴヤ,ナゴヤ";

        let temp_file = create_temp_csv(csv_content);
        let connections = create_mock_connections();

        let user_dict =
            UserDictionary::new(temp_file.path(), UserDictFormat::Ipadic, connections).unwrap();

        // Test lookup for "東京" - should return 2 entries
        let results_tokyo = user_dict.lookup("東京").unwrap();
        assert_eq!(results_tokyo.len(), 2);

        // Test lookup for "大阪" - should return 1 entry
        let results_osaka = user_dict.lookup("大阪").unwrap();
        assert_eq!(results_osaka.len(), 1);
        assert_eq!(results_osaka[0].surface, "大阪");
        assert_eq!(results_osaka[0].reading, "オオサカ");
        assert_eq!(results_osaka[0].phonetic, "オーサカ");

        // Test lookup for "名古屋" - should return 1 entry
        let results_nagoya = user_dict.lookup("名古屋").unwrap();
        assert_eq!(results_nagoya.len(), 1);
        assert_eq!(results_nagoya[0].surface, "名古屋");
        assert_eq!(results_nagoya[0].reading, "ナゴヤ");

        // Test lookup for non-existent entry
        let results_none = user_dict.lookup("京都").unwrap();
        assert_eq!(results_none.len(), 0);
    }

    #[test]
    fn test_dictionary_lookup_empty_string() {
        let csv_content = "\
東京,1288,1288,4569,名詞,固有名詞,地域,一般,*,*,東京,トウキョウ,トウキョウ";

        let temp_file = create_temp_csv(csv_content);
        let connections = create_mock_connections();

        let user_dict =
            UserDictionary::new(temp_file.path(), UserDictFormat::Ipadic, connections).unwrap();

        // Test lookup with empty string
        let results = user_dict.lookup("").unwrap();
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_dictionary_lookup_utf8_characters() {
        let csv_content = "\
こんにちは,1,1,100,感動詞,*,*,*,*,*,こんにちは,コンニチワ,コンニチワ
カタカナ,2,2,200,名詞,一般,*,*,*,*,カタカナ,カタカナ,カタカナ
漢字,3,3,300,名詞,一般,*,*,*,*,漢字,カンジ,カンジ";

        let temp_file = create_temp_csv(csv_content);
        let connections = create_mock_connections();

        let user_dict =
            UserDictionary::new(temp_file.path(), UserDictFormat::Ipadic, connections).unwrap();

        // Test lookup for different UTF-8 character types
        let results_hiragana = user_dict.lookup("こんにちは").unwrap();
        assert_eq!(results_hiragana.len(), 1);
        assert_eq!(results_hiragana[0].surface, "こんにちは");
        assert_eq!(results_hiragana[0].reading, "コンニチワ");

        let results_katakana = user_dict.lookup("カタカナ").unwrap();
        assert_eq!(results_katakana.len(), 1);
        assert_eq!(results_katakana[0].surface, "カタカナ");

        let results_kanji = user_dict.lookup("漢字").unwrap();
        assert_eq!(results_kanji.len(), 1);
        assert_eq!(results_kanji[0].surface, "漢字");
        assert_eq!(results_kanji[0].reading, "カンジ");
    }

    #[test]
    fn test_dictionary_lookup_simpledic_format() {
        let csv_content = "\
東京スカイツリー,カスタム名詞,トウキョウスカイツリー
東武スカイツリーライン,カスタム名詞,トウブスカイツリーライン";

        let temp_file = create_temp_csv(csv_content);
        let connections = create_mock_connections();

        let user_dict =
            UserDictionary::new(temp_file.path(), UserDictFormat::Simpledic, connections).unwrap();

        // Test lookup for simplified format entries
        let results = user_dict.lookup("東京スカイツリー").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].surface, "東京スカイツリー");
        assert_eq!(results[0].left_id, 0);
        assert_eq!(results[0].right_id, 0);
        assert_eq!(results[0].cost, -32000);
        assert_eq!(results[0].part_of_speech, "カスタム名詞,*,*,*");
        assert_eq!(results[0].inflection_type, "*");
        assert_eq!(results[0].inflection_form, "*");
        assert_eq!(results[0].base_form, "東京スカイツリー");
        assert_eq!(results[0].reading, "トウキョウスカイツリー");
        assert_eq!(results[0].phonetic, "トウキョウスカイツリー");
    }
}

#[cfg(test)]
mod connection_cost_tests {
    use super::*;
    use crate::dictionary::Dictionary;

    #[test]
    fn test_get_trans_cost_valid_ids() {
        let csv_content = "\
東京,1288,1288,4569,名詞,固有名詞,地域,一般,*,*,東京,トウキョウ,トウキョウ";

        let temp_file = create_temp_csv(csv_content);
        let connections = create_mock_connections();

        let user_dict =
            UserDictionary::new(temp_file.path(), UserDictFormat::Ipadic, connections).unwrap();

        // Test valid connection costs from our mock 3x3 matrix
        assert_eq!(user_dict.get_trans_cost(0, 0).unwrap(), 0);
        assert_eq!(user_dict.get_trans_cost(0, 1).unwrap(), 1);
        assert_eq!(user_dict.get_trans_cost(0, 2).unwrap(), 2);
        assert_eq!(user_dict.get_trans_cost(1, 0).unwrap(), 1);
        assert_eq!(user_dict.get_trans_cost(1, 1).unwrap(), 0);
        assert_eq!(user_dict.get_trans_cost(1, 2).unwrap(), 3);
        assert_eq!(user_dict.get_trans_cost(2, 0).unwrap(), 2);
        assert_eq!(user_dict.get_trans_cost(2, 1).unwrap(), 3);
        assert_eq!(user_dict.get_trans_cost(2, 2).unwrap(), 0);
    }

    #[test]
    fn test_get_trans_cost_invalid_left_id() {
        let csv_content = "\
東京,1288,1288,4569,名詞,固有名詞,地域,一般,*,*,東京,トウキョウ,トウキョウ";

        let temp_file = create_temp_csv(csv_content);
        let connections = create_mock_connections();

        let user_dict =
            UserDictionary::new(temp_file.path(), UserDictFormat::Ipadic, connections).unwrap();

        // Test invalid left_id (out of bounds)
        let result = user_dict.get_trans_cost(999, 0);
        assert!(result.is_err());
        match result.unwrap_err() {
            RunomeError::InvalidConnectionId { left_id, right_id } => {
                assert_eq!(left_id, 999);
                assert_eq!(right_id, 0);
            }
            _ => panic!("Expected InvalidConnectionId error"),
        }
    }

    #[test]
    fn test_get_trans_cost_invalid_right_id() {
        let csv_content = "\
東京,1288,1288,4569,名詞,固有名詞,地域,一般,*,*,東京,トウキョウ,トウキョウ";

        let temp_file = create_temp_csv(csv_content);
        let connections = create_mock_connections();

        let user_dict =
            UserDictionary::new(temp_file.path(), UserDictFormat::Ipadic, connections).unwrap();

        // Test invalid right_id (out of bounds)
        let result = user_dict.get_trans_cost(0, 999);
        assert!(result.is_err());
        match result.unwrap_err() {
            RunomeError::InvalidConnectionId { left_id, right_id } => {
                assert_eq!(left_id, 0);
                assert_eq!(right_id, 999);
            }
            _ => panic!("Expected InvalidConnectionId error"),
        }
    }

    #[test]
    fn test_get_trans_cost_both_invalid_ids() {
        let csv_content = "\
東京,1288,1288,4569,名詞,固有名詞,地域,一般,*,*,東京,トウキョウ,トウキョウ";

        let temp_file = create_temp_csv(csv_content);
        let connections = create_mock_connections();

        let user_dict =
            UserDictionary::new(temp_file.path(), UserDictFormat::Ipadic, connections).unwrap();

        // Test both invalid IDs
        let result = user_dict.get_trans_cost(999, 888);
        assert!(result.is_err());
        match result.unwrap_err() {
            RunomeError::InvalidConnectionId { left_id, right_id } => {
                assert_eq!(left_id, 999);
                assert_eq!(right_id, 888);
            }
            _ => panic!("Expected InvalidConnectionId error"),
        }
    }
}
