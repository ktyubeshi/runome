"""
Type stubs for runome Rust module.
"""

from typing import Iterable, Iterator, Optional, Sequence, Union, Tuple

TokenResult = Union["Token", str, Tuple[str, int]]

class Token:
    """Token with morphological information."""

    @property
    def surface(self) -> str:
        """Surface form of the token."""
        ...

    @property
    def part_of_speech(self) -> str:
        """Part of speech information."""
        ...

    @property
    def infl_type(self) -> str:
        """Inflection type."""
        ...

    @property
    def infl_form(self) -> str:
        """Inflection form."""
        ...

    @property
    def base_form(self) -> str:
        """Base form of the token."""
        ...

    @property
    def reading(self) -> str:
        """Reading of the token."""
        ...

    @property
    def phonetic(self) -> str:
        """Phonetic transcription."""
        ...

    @property
    def node_type(self) -> str:
        """Type of the node (SysDict, UserDict, Unknown)."""
        ...

    def __str__(self) -> str:
        """String representation in Janome format."""
        ...

    def __repr__(self) -> str:
        """Debug representation."""
        ...

class TokenIterator:
    """Iterator for tokenization results."""

    def __iter__(self) -> "TokenIterator":
        """Return self as iterator."""
        ...

    def __next__(self) -> Union[Token, str]:
        """Return next token or surface string."""
        ...


class CharFilter:
    """Base class for character filters."""

    def apply(self, text: str) -> str:
        """Apply the filter to input text."""
        ...

    def __call__(self, text: str) -> str:
        """Allow the filter to be called directly."""
        ...


class RegexReplaceCharFilter(CharFilter):
    """Character filter performing regex-based replacements."""

    def __init__(self, pattern: str, replacement: str) -> None:
        ...


class UnicodeNormalizeCharFilter(CharFilter):
    """Character filter applying Unicode normalization."""

    def __init__(self, form: str = "NFKC") -> None:
        ...


class TokenFilterIterator:
    """Iterator for token filter results."""

    def __iter__(self) -> "TokenFilterIterator":
        ...

    def __next__(self) -> TokenResult:
        ...


class TokenFilter:
    """Base class for token filters."""

    def apply(self, tokens: Iterable[Token]) -> TokenFilterIterator:
        ...

    def __call__(self, tokens: Iterable[Token]) -> TokenFilterIterator:
        ...


class LowerCaseFilter(TokenFilter):
    """Filter converting token surfaces to lowercase."""

    def __init__(self) -> None:
        ...


class UpperCaseFilter(TokenFilter):
    """Filter converting token surfaces to uppercase."""

    def __init__(self) -> None:
        ...


class POSStopFilter(TokenFilter):
    """Filter removing tokens with specified POS tags."""

    def __init__(self, pos_list: Sequence[str]) -> None:
        ...


class POSKeepFilter(TokenFilter):
    """Filter keeping only tokens with specified POS tags."""

    def __init__(self, pos_list: Sequence[str]) -> None:
        ...


class CompoundNounFilter(TokenFilter):
    """Filter combining consecutive nouns."""

    def __init__(self) -> None:
        ...


class ExtractAttributeFilter(TokenFilter):
    """Filter extracting a single attribute from each token."""

    def __init__(self, attr: str = "surface") -> None:
        ...


class TokenCountFilter(TokenFilter):
    """Terminal filter counting token occurrences."""

    def __init__(self, attr: str = "surface", sorted: bool = False) -> None:
        ...


class Analyzer:
    """Analyzer orchestrating char filters, tokenizer, and token filters."""

    def __init__(
        self,
        *,
        char_filters: Optional[Sequence[CharFilter]] = None,
        tokenizer: Optional[Tokenizer] = None,
        token_filters: Optional[Sequence[TokenFilter]] = None,
    ) -> None:
        ...

    def analyze(self, text: str) -> TokenFilterIterator:
        ...

class Tokenizer:
    """Japanese morphological analyzer."""

    def __init__(
        self,
        udic: str = "",
        *,
        udic_enc: str = "utf8",
        udic_type: str = "ipadic",
        max_unknown_length: int = 1024,
        wakati: bool = False,
    ) -> None:
        """Initialize tokenizer.

        Args:
            udic: User dictionary file path (CSV format) or directory path to compiled dictionary data (default: '')
            udic_enc: Character encoding for user dictionary - 'utf8', 'euc-jp', or 'shift_jis' (default: 'utf8')
            udic_type: User dictionary type - 'ipadic' or 'simpledic' (default: 'ipadic')
            max_unknown_length: Maximum unknown word length (default: 1024)
            wakati: Wakati mode flag (default: False)
        """
        ...

    def tokenize(
        self, text: str, wakati: Optional[bool] = None, baseform_unk: bool = True
    ) -> TokenIterator:
        """Tokenize text.

        Args:
            text: Input text to tokenize
            wakati: Override wakati mode (default: None)
            baseform_unk: Set base form for unknown words (default: True)

        Returns:
            Iterator yielding Token objects (wakati=False) or strings (wakati=True)
        """
        ...
