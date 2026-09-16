---
mdt: fix
---

# Keep tag spans byte-accurate when string arguments decode escapes

Token groups derived their end offset from each token's `Display` output, which re-renders a decoded string shorter than its raw source text. Once escape decoding landed, any tag carrying an escape such as `indent:"\t"` computed an opening span one byte short, so `mdt update` spliced the tag's closing `>` out of the file and `mdt check` could no longer match the consumer. Token group ends are now derived from the raw source slice the lexer consumes, keeping offsets byte-accurate regardless of decoded value length.
