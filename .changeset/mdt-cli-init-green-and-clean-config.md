---
"mdt_cli": fix
---

# `mdt init` leaves a green project and clean config

The sample `readme.md` created by `mdt init` now contains the sample provider's content instead of a placeholder, so a freshly initialized project passes `mdt check` immediately. The generated annotated `mdt.toml` is also rebuilt: the mangled glob examples (`src/**/_.ts`, `packages/_/readme.md`) are restored to `src/**/*.ts` and `packages/*/readme.md`, and the doubled blank lines introduced by markdown formatting of the template source are gone.
