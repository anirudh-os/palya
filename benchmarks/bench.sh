#!/usr/bin/env bash
# Set PALYA to the Palya binary path when it is not available on PATH.

PALYA="PATH_TO_PALYA"

# Generate the benchmark corpus when missing.
if [ ! -d "ssg_benchmark" ]; then
  echo "ssg_benchmark/ not found; generating corpus..."
  python3 "$(dirname "$0")/create.py"
fi

# Cold build benchmark.
hyperfine --warmup 1 --runs 10 \
  --prepare "rm -rf ssg_benchmark/palya/dist" \
    "$PALYA --input ssg_benchmark/palya --output ssg_benchmark/palya/dist" \
  --prepare "rm -rf ssg_benchmark/hugo/public" \
    "hugo --source ssg_benchmark/hugo --destination ssg_benchmark/hugo/public"

# Report output file counts.
PALYA_COUNT=$(find ssg_benchmark/palya/dist  -name '*.html' | wc -l | xargs)
HUGO_COUNT=$(find ssg_benchmark/hugo/public  -name '*.html' | wc -l | xargs)
echo "HTML files rendered - Palya: $PALYA_COUNT  Hugo: $HUGO_COUNT"

# Palya incremental build benchmark (single-file change).
# Hugo performs a full rebuild per CLI invocation, so this is reported as
# a Palya-only data point.
PALYA_TEST_FILE=$(find ssg_benchmark/palya/content -name '*.md' | head -n 1)

hyperfine --warmup 1 --runs 10 \
  --prepare "rm -rf ssg_benchmark/palya/dist && $PALYA --input ssg_benchmark/palya --output ssg_benchmark/palya/dist && echo ' ' >> $PALYA_TEST_FILE" \
    "$PALYA --input ssg_benchmark/palya --output ssg_benchmark/palya/dist"