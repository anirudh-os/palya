#!/usr/bin/env bash
# Run this script from the directory that contains `ssg_benchmark/`.
# Set the `PALYA` variable below if the binary is not on your PATH.

PALYA="PATH_TO_PALYA" # Use an absolute path when `palya` is not available on PATH.

# -----------------------------------------------------------------------------
# Cold build benchmark
#
# Removes output directories before each run to measure full parse + render time.
# -----------------------------------------------------------------------------
hyperfine --warmup 1 --runs 5 \
  --prepare "rm -rf ssg_benchmark/palya/dist" \
    "$PALYA --input ssg_benchmark/palya --output ssg_benchmark/palya/dist" \
  --prepare "rm -rf ssg_benchmark/hugo/public" \
    "hugo --source ssg_benchmark/hugo --destination ssg_benchmark/hugo/public" 

# -----------------------------------------------------------------------------
# Incremental build benchmark (single file change)
#
# Select one markdown file to modify during each prepare step.
# -----------------------------------------------------------------------------
PALYA_TEST_FILE=$(find ssg_benchmark/palya/content -name '*.md' | head -n 1)

hyperfine --warmup 1 --runs 5 \
  --prepare "rm -rf ssg_benchmark/palya/dist && PATH_TO_PALYA --input ssg_benchmark/palya --output ssg_benchmark/palya/dist && echo ' ' >> $PALYA_TEST_FILE" \
    "PATH_TO_PALYA --input ssg_benchmark/palya --output ssg_benchmark/palya/dist" \
