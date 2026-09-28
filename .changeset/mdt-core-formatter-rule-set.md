"mdt_core": feat
---

# Add `FormatterRuleSet` for precompiled formatter globs

`FormatterRuleSet::compile` turns a formatter's `patterns` or `ignore` list into reusable glob matchers with gitignore-style ordered `!` negation semantics, and `FormatterConfig::matches_file` keeps its behavior. Check and update runs compile the rules once per run instead of recompiling every glob for every scanned file.
