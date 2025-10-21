"""
Test cases for Python API compatibility with Janome.

This test suite includes:
1. Basic Python binding functionality tests
2. Equivalent tests from Janome's test_tokenizer.py
3. API compatibility verification
"""

import os
from pathlib import Path

import pytest
from runome.tokenizer import Tokenizer, Token

FIXTURES_ROOT = Path(__file__).resolve().parent.parent / "fixtures"
USERDIC_DIR = FIXTURES_ROOT / "userdic"

class TestBasicPythonBinding:
    """Test basic Python binding functionality."""

    def test_tokenizer_creation(self):
        """Test basic tokenizer creation."""
        tokenizer = Tokenizer()
        assert tokenizer is not None

    def test_tokenizer_with_params(self):
        """Test tokenizer creation with parameters."""
        tokenizer = Tokenizer(max_unknown_length=2048, wakati=True)
        assert tokenizer is not None

    def test_tokenizer_user_dict_invalid_file(self):
        """Test that invalid user dictionary file raises appropriate error."""
        with pytest.raises(Exception):
            Tokenizer(udic="nonexistent.csv")

    def test_basic_tokenization(self):
        """Test basic tokenization returns tokens."""
        tokenizer = Tokenizer()
        tokens = list(tokenizer.tokenize("テスト"))
        assert len(tokens) > 0
        assert all(isinstance(token, Token) for token in tokens)

    def test_wakati_mode(self):
        """Test wakati mode returns strings."""
        tokenizer = Tokenizer()
        tokens = list(tokenizer.tokenize("テスト", wakati=True))
        assert len(tokens) > 0
        assert all(isinstance(token, str) for token in tokens)

    def test_empty_text(self):
        """Test tokenization of empty string."""
        tokenizer = Tokenizer()
        tokens = list(tokenizer.tokenize(""))
        assert len(tokens) == 0

    def test_token_properties(self):
        """Test all token properties are accessible."""
        tokenizer = Tokenizer()
        tokens = list(tokenizer.tokenize("テスト"))
        assert len(tokens) > 0

        token = tokens[0]

        # Test all properties exist and return strings
        assert isinstance(token.surface, str)
        assert isinstance(token.part_of_speech, str)
        assert isinstance(token.infl_type, str)
        assert isinstance(token.infl_form, str)
        assert isinstance(token.base_form, str)
        assert isinstance(token.reading, str)
        assert isinstance(token.phonetic, str)
        assert isinstance(token.node_type, str)

        # Test surface is not empty
        assert len(token.surface) > 0

        # Test part_of_speech is not empty
        assert len(token.part_of_speech) > 0

    def test_token_string_representation(self):
        """Test token string representation matches Janome format."""
        tokenizer = Tokenizer()
        tokens = list(tokenizer.tokenize("テスト"))
        assert len(tokens) > 0

        token = tokens[0]
        str_repr = str(token)

        # Should contain tab separator
        assert "\t" in str_repr

        # Should start with surface
        assert str_repr.startswith(token.surface)

        # Should contain comma-separated morphological info
        parts = str_repr.split("\t")
        assert len(parts) == 2
        assert "," in parts[1]

    def test_token_repr(self):
        """Test token repr representation."""
        tokenizer = Tokenizer()
        tokens = list(tokenizer.tokenize("テスト"))
        assert len(tokens) > 0

        token = tokens[0]
        repr_str = repr(token)

        # Should contain Token and surface
        assert "Token" in repr_str
        assert token.surface in repr_str

    def test_iterator_protocol(self):
        """Test that tokenize returns proper iterator."""
        tokenizer = Tokenizer()
        result = tokenizer.tokenize("テスト")

        # Should be iterable
        iterator = iter(result)

        # Should support next()
        first_token = next(iterator)
        assert isinstance(first_token, Token)

        # Should raise StopIteration when exhausted
        tokens = list(tokenizer.tokenize(""))
        assert len(tokens) == 0


class TestJanomeEquivalent:
    """Test cases equivalent to Janome's test_tokenizer.py."""

    def setup_method(self):
        """Setup tokenizer for each test."""
        self.tokenizer = Tokenizer()

    def _check_token(self, token, surface, detail, node_type_str):
        """Helper method to check token properties (equivalent to Janome's _check_token)."""
        assert token.surface == surface

        # Reconstruct detail string from token properties
        detail_parts = [
            token.part_of_speech,
            token.infl_type,
            token.infl_form,
            token.base_form,
            token.reading,
            token.phonetic,
        ]
        reconstructed_detail = ",".join(detail_parts)
        assert reconstructed_detail == detail

        # Check string representation
        expected_str = f"{surface}\t{detail}"
        assert str(token) == expected_str

        # Check node type (convert to string for comparison)
        assert node_type_str.lower() in token.node_type.lower()

    def test_tokenize_basic(self):
        """Test basic tokenization (equivalent to test_tokenize_nommap)."""
        text = "すもももももももものうち"
        tokens = list(self.tokenizer.tokenize(text))
        assert len(tokens) == 7

        # Check each token (using relaxed assertions for now)
        assert tokens[0].surface == "すもも"
        assert tokens[1].surface == "も"
        assert tokens[2].surface == "もも"
        assert tokens[3].surface == "も"
        assert tokens[4].surface == "もも"
        assert tokens[5].surface == "の"
        assert tokens[6].surface == "うち"

        # Check that all are system dictionary tokens
        for token in tokens:
            assert "sys" in token.node_type.lower() or "dict" in token.node_type.lower()

    def test_tokenize_unicode_unknown(self):
        """Test tokenization with unicode unknown characters (equivalent to test_tokenize2)."""
        text = "𠮷野屋"
        tokens = list(self.tokenizer.tokenize(text))
        assert len(tokens) == 3

        # First token should be unknown
        assert tokens[0].surface == "𠮷"
        assert "unknown" in tokens[0].node_type.lower()

        # Other tokens should be from system dictionary
        assert tokens[1].surface == "野"
        assert tokens[2].surface == "屋"

        # Test Korean text
        text = "한국어"
        tokens = list(self.tokenizer.tokenize(text))
        assert len(tokens) == 1
        assert tokens[0].surface == "한국어"
        assert "unknown" in tokens[0].node_type.lower()

    def test_tokenize_unknown_numbers(self):
        """Test tokenization with unknown numbers (equivalent to test_tokenize_unknown)."""
        text = "2009年10月16日"
        tokens = list(self.tokenizer.tokenize(text))
        assert len(tokens) == 6

        # Check surfaces
        assert tokens[0].surface == "2009"
        assert tokens[1].surface == "年"
        assert tokens[2].surface == "10"
        assert tokens[3].surface == "月"
        assert tokens[4].surface == "16"
        assert tokens[5].surface == "日"

        # Check node types
        assert "unknown" in tokens[0].node_type.lower()  # 2009
        assert "unknown" in tokens[2].node_type.lower()  # 10
        assert "unknown" in tokens[4].node_type.lower()  # 16

    def test_tokenize_complex_unknown(self):
        """Test complex text with unknown words (equivalent to test_tokenize_unknown part 2)."""
        text = "マルチメディア放送（VHF-HIGH帯）「モバキャス」"
        tokens = list(self.tokenizer.tokenize(text))
        assert len(tokens) == 11

        # Check some key tokens
        assert tokens[0].surface == "マルチメディア"
        assert tokens[1].surface == "放送"
        assert tokens[2].surface == "（"
        assert tokens[3].surface == "VHF"
        assert tokens[4].surface == "-"
        assert tokens[5].surface == "HIGH"
        assert tokens[6].surface == "帯"
        assert tokens[7].surface == "）"
        assert tokens[8].surface == "「"
        assert tokens[9].surface == "モバキャス"
        assert tokens[10].surface == "」"

        # Check that VHF, -, HIGH, モバキャス are unknown
        assert "unknown" in tokens[3].node_type.lower()  # VHF
        assert "unknown" in tokens[4].node_type.lower()  # -
        assert "unknown" in tokens[5].node_type.lower()  # HIGH
        assert "unknown" in tokens[9].node_type.lower()  # モバキャス

    def test_tokenize_unknown_no_baseform(self):
        """Test tokenization with baseform_unk=False (equivalent to test_tokenize_unknown_no_baseform)."""
        text = "2009年10月16日"
        tokens = list(self.tokenizer.tokenize(text, baseform_unk=False))
        assert len(tokens) == 6

        # Check that unknown words have "*" as base form
        assert tokens[0].surface == "2009"
        assert tokens[0].base_form == "*"  # baseform_unk=False
        assert tokens[2].surface == "10"
        assert tokens[2].base_form == "*"  # baseform_unk=False
        assert tokens[4].surface == "16"
        assert tokens[4].base_form == "*"  # baseform_unk=False

        # System dictionary tokens should still have proper base forms
        assert tokens[1].surface == "年"
        assert tokens[1].base_form == "年"

    def test_tokenize_wakati_mode(self):
        """Test wakati mode (equivalent to test_tokenize_wakati)."""
        text = "すもももももももものうち"
        tokenizer = Tokenizer(wakati=True)
        tokens = list(tokenizer.tokenize(text, wakati=True))
        assert len(tokens) == 7

        # In wakati mode, all tokens should be strings
        assert all(isinstance(token, str) for token in tokens)

        # Check surfaces
        assert tokens[0] == "すもも"
        assert tokens[1] == "も"
        assert tokens[2] == "もも"
        assert tokens[3] == "も"
        assert tokens[4] == "もも"
        assert tokens[5] == "の"
        assert tokens[6] == "うち"

    def test_tokenize_wakati_mode_only(self):
        """Test wakati mode when initialized with wakati=True (equivalent to test_tokenize_wakati_mode_only)."""
        text = "すもももももももものうち"
        tokenizer = Tokenizer(wakati=True)
        tokens = list(tokenizer.tokenize(text, wakati=False))

        # When tokenizer is initialized with wakati=True, wakati=False parameter should be ignored
        assert len(tokens) == 7
        print(tokens)
        assert all(isinstance(token, str) for token in tokens)

        # Check surfaces
        assert tokens[0] == "すもも"
        assert tokens[1] == "も"
        assert tokens[2] == "もも"
        assert tokens[3] == "も"
        assert tokens[4] == "もも"
        assert tokens[5] == "の"
        assert tokens[6] == "うち"

    def test_baseform_unk_parameter(self):
        """Test baseform_unk parameter works correctly."""
        text = "2009年"

        # Test with baseform_unk=True (default)
        tokens_true = list(self.tokenizer.tokenize(text, baseform_unk=True))

        # Test with baseform_unk=False
        tokens_false = list(self.tokenizer.tokenize(text, baseform_unk=False))

        # Both should return same number of tokens
        assert len(tokens_true) == len(tokens_false)

        # Both should have same surface
        assert tokens_true[0].surface == tokens_false[0].surface == "2009"

        # But different base forms for unknown words
        assert tokens_true[0].base_form == "2009"  # baseform_unk=True
        assert tokens_false[0].base_form == "*"  # baseform_unk=False

    def test_multiple_character_types(self):
        """Test various character types."""
        test_cases = [
            ("2009", "numeric"),
            ("ABC", "alphabetic"),
            ("すもも", "hiragana"),
            ("テスト", "katakana"),
        ]

        for text, char_type in test_cases:
            tokens = list(self.tokenizer.tokenize(text))
            assert len(tokens) >= 1, f"Failed to tokenize {char_type} text: {text}"
            assert tokens[0].surface == text, (
                f"Surface mismatch for {char_type} text: {text}"
            )


class TestCompatibility:
    """Test compatibility with Janome API."""

    def test_parameter_compatibility(self):
        """Test parameter compatibility with Janome."""
        # Test all supported parameters
        tokenizer = Tokenizer(
            udic="",  # Empty string should work
            max_unknown_length=2048,
            wakati=False,
        )

        tokens = list(tokenizer.tokenize("テスト"))
        assert len(tokens) > 0

    def test_tokenize_parameters(self):
        """Test tokenize method parameters."""
        tokenizer = Tokenizer()

        # Test with all parameters
        tokens = list(tokenizer.tokenize("テスト", wakati=False, baseform_unk=True))
        assert len(tokens) > 0
        assert all(isinstance(token, Token) for token in tokens)

        # Test with wakati=True
        tokens = list(tokenizer.tokenize("テスト", wakati=True, baseform_unk=True))
        assert len(tokens) > 0
        assert all(isinstance(token, str) for token in tokens)


class TestUserDictionary:
    """Test user dictionary functionality."""

    def setup_method(self):
        """Setup test data paths."""
        self.user_ipadic_path = str(USERDIC_DIR / "user_ipadic.csv")
        self.user_simpledic_path = str(USERDIC_DIR / "user_simpledic.csv")
        self.user_ipadic_eucjp_path = str(USERDIC_DIR / "user_ipadic_eucjp.csv")
        self.user_ipadic_sjis_path = str(USERDIC_DIR / "user_ipadic_sjis.csv")

    def test_user_dict_ipadic_creation(self):
        """Test creating tokenizer with IPADIC format user dictionary."""
        tokenizer = Tokenizer(udic=self.user_ipadic_path, udic_type="ipadic")
        assert tokenizer is not None

    def test_user_dict_simpledic_creation(self):
        """Test creating tokenizer with Simpledic format user dictionary."""
        tokenizer = Tokenizer(udic=self.user_simpledic_path, udic_type="simpledic")
        assert tokenizer is not None

    def test_user_dict_ipadic_tokenization(self):
        """Test tokenization with IPADIC format user dictionary."""
        tokenizer = Tokenizer(udic=self.user_ipadic_path, udic_type="ipadic")

        # Test text from Janome examples
        text = "東京スカイツリーへのお越しは、東武スカイツリーライン「とうきょうスカイツリー駅」が便利です。"
        tokens = list(tokenizer.tokenize(text))

        # Find user dictionary tokens
        user_dict_tokens = [
            token for token in tokens if "user" in token.node_type.lower()
        ]
        assert (
            len(user_dict_tokens) >= 3
        )  # Should have at least the 3 user dict entries

        # Check specific user dictionary tokens
        surfaces = [token.surface for token in tokens]
        assert "東京スカイツリー" in surfaces
        assert "東武スカイツリーライン" in surfaces
        assert "とうきょうスカイツリー駅" in surfaces

        # Check that user dictionary tokens have correct part of speech
        for token in tokens:
            if token.surface == "東京スカイツリー":
                assert token.part_of_speech == "名詞,固有名詞,一般,*"
                assert "user" in token.node_type.lower()
                break

    def test_user_dict_simpledic_tokenization(self):
        """Test tokenization with Simpledic format user dictionary."""
        tokenizer = Tokenizer(udic=self.user_simpledic_path, udic_type="simpledic")

        # Test text from Janome examples
        text = "東京スカイツリーへのお越しは、東武スカイツリーライン「とうきょうスカイツリー駅」が便利です。"
        tokens = list(tokenizer.tokenize(text))

        # Find user dictionary tokens
        user_dict_tokens = [
            token for token in tokens if "user" in token.node_type.lower()
        ]
        assert (
            len(user_dict_tokens) >= 3
        )  # Should have at least the 3 user dict entries

        # Check that user dictionary tokens have correct part of speech (from simpledic)
        for token in tokens:
            if token.surface == "東京スカイツリー":
                assert token.part_of_speech == "カスタム名詞,*,*,*"
                assert "user" in token.node_type.lower()
                break

    def test_user_dict_vs_no_user_dict(self):
        """Test difference between using user dict and not using it."""
        # Tokenizer without user dictionary
        tokenizer_normal = Tokenizer()
        tokens_normal = list(tokenizer_normal.tokenize("東京スカイツリー"))

        # Tokenizer with user dictionary
        tokenizer_user = Tokenizer(udic=self.user_ipadic_path, udic_type="ipadic")
        tokens_user = list(tokenizer_user.tokenize("東京スカイツリー"))

        # Without user dict, this should be broken into multiple tokens
        assert len(tokens_normal) > 1

        # With user dict, this should be a single token
        assert len(tokens_user) == 1
        assert tokens_user[0].surface == "東京スカイツリー"
        assert "user" in tokens_user[0].node_type.lower()

    def test_user_dict_invalid_type(self):
        """Test invalid user dictionary type."""
        with pytest.raises(Exception, match="Unsupported user dictionary type"):
            Tokenizer(udic=self.user_ipadic_path, udic_type="invalid")

    def test_user_dict_unsupported_encoding(self):
        """Test unsupported encoding."""
        with pytest.raises(Exception, match="Unsupported encoding"):
            Tokenizer(udic=self.user_ipadic_path, udic_enc="iso-8859-1")

    def test_user_dict_euc_jp_encoding(self):
        """Test EUC-JP encoding support."""
        tokenizer = Tokenizer(
            udic=self.user_ipadic_eucjp_path, udic_type="ipadic", udic_enc="euc-jp"
        )
        tokens = list(tokenizer.tokenize("東京スカイツリー"))

        assert len(tokens) == 1
        assert tokens[0].surface == "東京スカイツリー"
        assert tokens[0].part_of_speech == "名詞,固有名詞,一般,*"
        assert "user" in tokens[0].node_type.lower()

    def test_user_dict_shift_jis_encoding(self):
        """Test Shift_JIS encoding support."""
        tokenizer = Tokenizer(
            udic=self.user_ipadic_sjis_path, udic_type="ipadic", udic_enc="shift_jis"
        )
        tokens = list(tokenizer.tokenize("東京スカイツリー"))

        assert len(tokens) == 1
        assert tokens[0].surface == "東京スカイツリー"
        assert tokens[0].part_of_speech == "名詞,固有名詞,一般,*"
        assert "user" in tokens[0].node_type.lower()

    def test_user_dict_encoding_variations(self):
        """Test different encoding name variations."""
        # Test utf-8 variant
        tokenizer1 = Tokenizer(
            udic=self.user_ipadic_path, udic_type="ipadic", udic_enc="utf-8"
        )
        tokens1 = list(tokenizer1.tokenize("東京スカイツリー"))
        assert len(tokens1) == 1

        # Test sjis variant
        tokenizer2 = Tokenizer(
            udic=self.user_ipadic_sjis_path, udic_type="ipadic", udic_enc="sjis"
        )
        tokens2 = list(tokenizer2.tokenize("東京スカイツリー"))
        assert len(tokens2) == 1

    def test_user_dict_wakati_mode(self):
        """Test user dictionary with wakati mode."""
        tokenizer = Tokenizer(udic=self.user_ipadic_path, udic_type="ipadic")

        # Test wakati mode
        tokens = list(tokenizer.tokenize("東京スカイツリー", wakati=True))
        assert len(tokens) == 1
        assert isinstance(tokens[0], str)
        assert tokens[0] == "東京スカイツリー"

    def test_tokenize_with_userdic(self):
        """Test tokenization with user dictionary (IPADIC format) - equivalent to Rust test_tokenize_with_userdic."""
        tokenizer = Tokenizer(udic=self.user_ipadic_path, udic_type="ipadic")

        # Test text from Janome examples - exactly the same as Rust test
        text = "東京スカイツリーへのお越しは、東武スカイツリーライン「とうきょうスカイツリー駅」が便利です。"
        tokens = list(tokenizer.tokenize(text))

        # Should produce exactly 14 tokens (same as Rust test)
        assert len(tokens) == 14, f"Expected 14 tokens but got {len(tokens)}"

        # Helper function to check token properties (equivalent to Rust check_token)
        def check_token(token, expected_surface, expected_detail, expected_node_type):
            assert token.surface == expected_surface, (
                f"Surface mismatch: expected '{expected_surface}', got '{token.surface}'"
            )

            # Reconstruct detail string from token properties
            detail_parts = [
                token.part_of_speech,
                token.infl_type,
                token.infl_form,
                token.base_form,
                token.reading,
                token.phonetic,
            ]
            actual_detail = ",".join(detail_parts)
            assert actual_detail == expected_detail, (
                f"Detail mismatch for '{expected_surface}': expected '{expected_detail}', got '{actual_detail}'"
            )

            # Check string representation
            expected_str = f"{expected_surface}\t{expected_detail}"
            assert str(token) == expected_str, (
                f"String representation mismatch for '{expected_surface}'"
            )

            # Check node type (case-insensitive contains check)
            assert expected_node_type.lower() in token.node_type.lower(), (
                f"Node type mismatch for '{expected_surface}': expected '{expected_node_type}' in '{token.node_type}'"
            )

        # Validate key tokens from user dictionary - same expectations as Rust test
        check_token(
            tokens[0],
            "東京スカイツリー",
            "名詞,固有名詞,一般,*,*,*,東京スカイツリー,トウキョウスカイツリー,トウキョウスカイツリー",
            "userdict",
        )
        check_token(tokens[1], "へ", "助詞,格助詞,一般,*,*,*,へ,ヘ,エ", "sysdict")
        check_token(tokens[2], "の", "助詞,連体化,*,*,*,*,の,ノ,ノ", "sysdict")
        check_token(
            tokens[3], "お越し", "名詞,一般,*,*,*,*,お越し,オコシ,オコシ", "sysdict"
        )
        check_token(tokens[4], "は", "助詞,係助詞,*,*,*,*,は,ハ,ワ", "sysdict")
        check_token(tokens[5], "、", "記号,読点,*,*,*,*,、,、,、", "sysdict")
        check_token(
            tokens[6],
            "東武スカイツリーライン",
            "名詞,固有名詞,一般,*,*,*,東武スカイツリーライン,トウブスカイツリーライン,トウブスカイツリーライン",
            "userdict",
        )
        check_token(tokens[7], "「", "記号,括弧開,*,*,*,*,「,「,「", "sysdict")
        check_token(
            tokens[8],
            "とうきょうスカイツリー駅",
            "名詞,固有名詞,一般,*,*,*,とうきょうスカイツリー駅,トウキョウスカイツリーエキ,トウキョウスカイツリーエキ",
            "userdict",
        )
        check_token(tokens[9], "」", "記号,括弧閉,*,*,*,*,」,」,」", "sysdict")
        check_token(tokens[10], "が", "助詞,格助詞,一般,*,*,*,が,ガ,ガ", "sysdict")
        check_token(
            tokens[11],
            "便利",
            "名詞,形容動詞語幹,*,*,*,*,便利,ベンリ,ベンリ",
            "sysdict",
        )
        check_token(
            tokens[12],
            "です",
            "助動詞,*,*,*,特殊・デス,基本形,です,デス,デス",
            "sysdict",
        )
        check_token(tokens[13], "。", "記号,句点,*,*,*,*,。,。,。", "sysdict")

        # Verify that user dictionary tokens are properly identified
        user_dict_tokens = [
            token for token in tokens if "user" in token.node_type.lower()
        ]
        assert len(user_dict_tokens) == 3, (
            f"Expected exactly 3 user dictionary tokens, got {len(user_dict_tokens)}"
        )

        # Check surfaces of user dictionary tokens
        user_surfaces = [token.surface for token in user_dict_tokens]
        expected_user_surfaces = [
            "東京スカイツリー",
            "東武スカイツリーライン",
            "とうきょうスカイツリー駅",
        ]
        assert user_surfaces == expected_user_surfaces, (
            f"User dictionary token surfaces mismatch: expected {expected_user_surfaces}, got {user_surfaces}"
        )

    def test_tokenize_with_simplified_userdic(self):
        """Test tokenization with simplified user dictionary - equivalent to Rust test_tokenize_with_simplified_userdic."""
        tokenizer = Tokenizer(udic=self.user_simpledic_path, udic_type="simpledic")

        # Test text from Janome examples - exactly the same as Rust test
        text = "東京スカイツリーへのお越しは、東武スカイツリーライン「とうきょうスカイツリー駅」が便利です。"
        tokens = list(tokenizer.tokenize(text))

        # Should produce exactly 14 tokens (same as Rust test)
        assert len(tokens) == 14, f"Expected 14 tokens but got {len(tokens)}"

        # Helper function to check token properties (equivalent to Rust check_token)
        def check_token(token, expected_surface, expected_detail, expected_node_type):
            assert token.surface == expected_surface, (
                f"Surface mismatch: expected '{expected_surface}', got '{token.surface}'"
            )

            # Reconstruct detail string from token properties
            detail_parts = [
                token.part_of_speech,
                token.infl_type,
                token.infl_form,
                token.base_form,
                token.reading,
                token.phonetic,
            ]
            actual_detail = ",".join(detail_parts)
            assert actual_detail == expected_detail, (
                f"Detail mismatch for '{expected_surface}': expected '{expected_detail}', got '{actual_detail}'"
            )

            # Check string representation
            expected_str = f"{expected_surface}\t{expected_detail}"
            assert str(token) == expected_str, (
                f"String representation mismatch for '{expected_surface}'"
            )

            # Check node type (case-insensitive contains check)
            assert expected_node_type.lower() in token.node_type.lower(), (
                f"Node type mismatch for '{expected_surface}': expected '{expected_node_type}' in '{token.node_type}'"
            )

        # Validate key tokens from user dictionary (simplified format) - same expectations as Rust test
        check_token(
            tokens[0],
            "東京スカイツリー",
            "カスタム名詞,*,*,*,*,*,東京スカイツリー,トウキョウスカイツリー,トウキョウスカイツリー",
            "userdict",
        )
        check_token(tokens[1], "へ", "助詞,格助詞,一般,*,*,*,へ,ヘ,エ", "sysdict")
        check_token(tokens[2], "の", "助詞,連体化,*,*,*,*,の,ノ,ノ", "sysdict")
        check_token(
            tokens[3], "お越し", "名詞,一般,*,*,*,*,お越し,オコシ,オコシ", "sysdict"
        )
        check_token(tokens[4], "は", "助詞,係助詞,*,*,*,*,は,ハ,ワ", "sysdict")
        check_token(tokens[5], "、", "記号,読点,*,*,*,*,、,、,、", "sysdict")
        check_token(
            tokens[6],
            "東武スカイツリーライン",
            "カスタム名詞,*,*,*,*,*,東武スカイツリーライン,トウブスカイツリーライン,トウブスカイツリーライン",
            "userdict",
        )
        check_token(tokens[7], "「", "記号,括弧開,*,*,*,*,「,「,「", "sysdict")
        check_token(
            tokens[8],
            "とうきょうスカイツリー駅",
            "カスタム名詞,*,*,*,*,*,とうきょうスカイツリー駅,トウキョウスカイツリーエキ,トウキョウスカイツリーエキ",
            "userdict",
        )
        check_token(tokens[9], "」", "記号,括弧閉,*,*,*,*,」,」,」", "sysdict")
        check_token(tokens[10], "が", "助詞,格助詞,一般,*,*,*,が,ガ,ガ", "sysdict")
        check_token(
            tokens[11],
            "便利",
            "名詞,形容動詞語幹,*,*,*,*,便利,ベンリ,ベンリ",
            "sysdict",
        )
        check_token(
            tokens[12],
            "です",
            "助動詞,*,*,*,特殊・デス,基本形,です,デス,デス",
            "sysdict",
        )
        check_token(tokens[13], "。", "記号,句点,*,*,*,*,。,。,。", "sysdict")

        # Verify that user dictionary tokens are properly identified
        user_dict_tokens = [
            token for token in tokens if "user" in token.node_type.lower()
        ]
        assert len(user_dict_tokens) == 3, (
            f"Expected exactly 3 user dictionary tokens, got {len(user_dict_tokens)}"
        )

        # Check surfaces of user dictionary tokens
        user_surfaces = [token.surface for token in user_dict_tokens]
        expected_user_surfaces = [
            "東京スカイツリー",
            "東武スカイツリーライン",
            "とうきょうスカイツリー駅",
        ]
        assert user_surfaces == expected_user_surfaces, (
            f"User dictionary token surfaces mismatch: expected {expected_user_surfaces}, got {user_surfaces}"
        )

        # Verify that simplified format tokens have the custom part of speech
        for token in user_dict_tokens:
            # For simplified format, the part_of_speech should start with "カスタム名詞"
            assert token.part_of_speech.startswith("カスタム名詞"), (
                f"Expected simplified format custom part of speech starting with 'カスタム名詞', got '{token.part_of_speech}' for token '{token.surface}'"
            )


if __name__ == "__main__":
    # Run tests
    pytest.main([__file__, "-v"])
