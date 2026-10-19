---
"mdt_core": minor
"mdt_cli": patch
"@m-d-t/skills": docs
---

# One padding default: omitted keys mean 0

`[padding]` had two defaults: an absent section meant `before = 0, after = 0`, but a present section with an omitted key defaulted to `1` (one blank line). `[padding]` with only `after = 2` silently meant `before = 1, after = 2` — the most-documented confusion point in mdt's config.

Now an omitted key behaves exactly like an absent section: content on the next line, no blank lines. Padding applies only when configured; write `before = 1` / `after = 1` explicitly when you want blank lines.

Configs that relied on the implicit `1` (a bare `[padding]` section, or a section listing only one key) must now state the values. The annotated `mdt.toml`, the configuration guide, and the agent skill are updated to the new rule.
