# Profiler Scripts

This directory contains profiling scripts to measure the performance of Runome tokenizer and compare it with Janome.

## Scripts

### run_cprofile.py

Performance profiling using Python's cProfile module.

**Usage:**
```bash
# Profile runome tokenizer
python run_cprofile.py

# Profile janome tokenizer for comparison
python run_cprofile.py --janome
```

**Output:**
- Prints profiling statistics to stdout
- Saves detailed profile data to `runome_tokenizer.profile` or `janome_tokenizer.profile`

### run_tracemalloc.py

Memory usage profiling using Python's tracemalloc module.

**Usage:**
```bash
# Profile runome tokenizer memory usage
python run_tracemalloc.py

# Profile janome tokenizer memory usage for comparison
python run_tracemalloc.py -janome
```

**Output:**
- Saves memory usage statistics to `runome_memusage.dump` or `janome_memusage.dump`

## Test Data

The scripts use `text_lemon.txt` as test data for tokenization. This file contains Japanese text for benchmarking purposes.

## Performance Comparison

To compare Runome vs Janome performance:

```bash
# Tox ensures a fresh virtualenv with deterministic deps
tox -e bench

# Quick benchmark (arguments are passed directly)
uv run bench --iterations 300 --warmup 10

# Profile both tokenizers
tox -e profile

uv run profile --repeat 20 --limit 30
uv run profile --janome --repeat 20 --limit 30

# Compare memory usage
python run_tracemalloc.py
python run_tracemalloc.py -janome
```

## Requirements

- Runome package installed (`pip install runome`)
- For comparison: Janome package installed (`pip install janome`)
