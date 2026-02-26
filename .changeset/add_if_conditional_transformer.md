---
mdt_core: minor
---

Add `if` conditional transformer for selectively including block content based on data values. The `if` transformer takes a dot-separated data path as an argument and includes the block content only when the referenced value is truthy (exists and is not false, null, empty string, or zero). Example usage: `<!-- {=block|if:"config.features.enabled"} -->`.
