# Fixtures Layout

This directory will store handcrafted inputs for the rebuilt test suite.

- `cases/`: Minimal and real-world text samples that will be fed to Janome and Runome.
- `userdic/`: User dictionary CSVs (both ipadic and simple formats) used for compatibility testing.

Populate these folders with UTF-8 encoded files. Large corpora should live outside the repo and be referenced through the golden generator script instead.
