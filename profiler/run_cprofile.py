"""Backward compatibleエントリポイント: runome.profile.main を利用する."""

from runome.profile import main


if __name__ == "__main__":
    raise SystemExit(main())
