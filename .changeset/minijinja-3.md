---
mdt_core: change
---

Render templates with minijinja 3. Trailing newlines still stay in the rendered block: that setting now lives on `SyntaxConfig` instead of `Environment`. Data contexts are passed through `Value::from(Serde(..))`, which needs minijinja's `serde` feature. Edge cases that minijinja changed to match Jinja2 (negative floor division, half-to-even `round`, and `title` after an apostrophe) follow the new engine. mdt does not set a fuel limit, so the new fuel accounting does not stop rendering.
