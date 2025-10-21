# Fixtures Layout

This directory will store handcrafted inputs for the rebuilt test suite.

- `cases/`: Minimal and real-world text samples that will be fed to Janome and Runome.
  - `basic_sumomo.txt`: Canonical 「すもももももももものうち」 smoke test。
  - `text_lemon.txt`: 文芸作品「檸檬」の冒頭。長文チャンク処理の回帰確認に利用。
  - `text_large.txt`: 日本語主体の長文パフォーマンステキスト。
  - `text_large_nonjp.txt`: 英数・記号が混在する長文テキスト。
- `userdic/`: User dictionary CSVs (both IPADIC and Simpledic formats) mirrored from the original Janome tests.
  - `user_ipadic.csv`: IPADICフォーマットの標準例。
  - `user_ipadic_eucjp.csv`: EUC-JP エンコード版。
  - `user_ipadic_sjis.csv`: Shift_JIS エンコード版。
  - `user_simpledic.csv`: Simpledicフォーマットの例。

Populate these folders with UTF-8 encoded files. Large corpora should live outside the repo and be referenced through the golden generator script instead.
