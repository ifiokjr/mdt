"mdt_core": perf
---

# Cut repeated work in scan, parse, and render hot paths

- The four grammar patterns used to classify token groups are now built once per thread instead of per HTML comment, eliminating hundreds of closure and string allocations for every comment scanned.
- Source-file comment extraction now locifies `<!--`/`-->` via `memchr` (SIMD) instead of a byte-window scan per offset — previously the dominant cost of scanning large non-markdown files.
- Provider templates without parameters render once per run instead of once per consumer, and the base data map is no longer deep-cloned per consumer.
- GFM parse options are constructed once per thread rather than per markdown file.
- Lenient comparisons skip whitespace normalization when bytes already match.
- The index cache artifact serializes as compact JSON instead of pretty JSON.
- File collection deduplication is set-based instead of a linear `contains` scan; comment extraction no longer over-allocates by file size.
