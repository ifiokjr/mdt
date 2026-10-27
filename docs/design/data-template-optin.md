# Per-provider template opt-in

## The problem

The moment a project configures `[data]`, **every** provider's content becomes a minijinja template. That is the right default for the data-driven projects `[data]` exists for, but it creates the biggest "silent failure" class mdt has:

- A provider that documents templating itself — install guides, mdt's own docs — contains literal `{{` and `{%`. Adding `[data]` (sometimes much later, for one unrelated namespace) either fails the render or, worse, renders wrong: `{{ version }}` in prose resolves to something unintended or empty.
- The escape hatch (`{% raw %}`) exists but nobody discovers it until the failure, and the failure message does not point at the two literal braces that caused it.
- The blast radius is project-wide: one namespace in `[data]` flips templating on for every `*.t.md` file, including shared `[templates] paths` providers that predate the config.

The docs spend the most words on this corner (raw-escaping, the format matrix, the watch-cache rules). This design removes the trap for content that was never meant to be templated, without breaking any existing project.

## Constraints

1. **`{@name:"param"}` already means something.** The quoted arguments after a provider name declare its parameters. So the obvious `{@name:raw}` suffix is not available: it would declare a parameter named `raw`.
2. Provider names are `[A-Za-z_][A-Za-z0-9_-]*`, case-sensitive, and any keyword we introduce is a legal name today. A syntax that reinterprets existing tags is a breaking change and is rejected here.
3. Consumers never see template syntax — they receive rendered bytes. Whatever we do changes provider tags only.
4. Semver discipline: mdt is pre-1.0 but the changelog theme is "no silent behavior changes". The default must stay _template-when-`[data]`-exists_ until a major.

## Candidate syntaxes

### A. Keyword prefix: `{@raw name}` — recommended

```markdown
<!-- {@raw badges} -->

Shields badges use `{{ github.repository }}` placeholders — literal text, never templated.

<!-- {/badges} -->
```

The close tag is unchanged (`{/name}`); the keyword sits **between the sigil and the name**:

- `{@raw badges}` — provider whose content is never templated, even when `[data]` exists.
- `{@template badges}` — the explicit opt-in twin, for readers who want symmetry (and for `default = "raw"` projects, below).
- `{@badges}` — unchanged: templated when `[data]` exists.

**Why it is unambiguous:** today `{@raw badges}` (two tokens before any quote) is a parse error, so no existing project uses it. `{@raw}` alone remains a provider _named_ `raw` — the keyword only applies when a second name token follows. The lexer change is small: after `@`, accept an optional `raw`/`template` keyword followed by a name.

**Diagnostics:** `{@template name}` with no `[data]` configured is fine (renders literally, as today). `{@raw name}` whose content contains `{% raw %}` is fine too — it is inert text. A new optional `[data] default = "template" | "raw"` (default `"template"`) lets a project invert the rule for itself: bare `{@name}` means raw, and data-driven providers say `{@template name}`. That kills the blast radius for docs-heavy repos without touching anyone else.

### B. Suffix `{@name:raw}` — rejected

Collides with parameter declarations (`{@name:"raw"}` declares a parameter; unquoted `raw` would need a new token type in the argument list and silently change meaning for scanners).

### C. Body marker (`<!-- template: off -->` first line inside the provider) — rejected

Invisible in the rendered file, easy to delete by accident, and it makes the tag no longer self-describing. mdt's whole UX leans on tags being the single source of truth.

## Quick win independent of syntax: better failure

Regardless of syntax rollout, the render error for literal-brace failures should improve first:

- When a provider fails to render (or renders empty) **and** its raw content contains `{{` or `{%`, the error should quote the offending span and say: _this provider contains literal template syntax; wrap it in `{% raw %}…{% endraw %}`, or mark the provider `{@raw …}`_.
- `mdt check` should emit a warning (not an error) when `[data]` exists and a provider contains `{{`/`{%` outside `{% raw %}` — most such providers predate the `[data]` config. Silence it per provider with `{@raw …}`, or globally with the existing `--ignore-*` style flag.

These two are shippable in a patch release and make the trap loud instead of silent while the syntax lands.

## Semantics summary

| Tag                | `[data]` absent | `[data]` present, `default = "template"` (today) | `default = "raw"` |
| ------------------ | --------------- | ------------------------------------------------ | ----------------- |
| `{@name}`          | literal         | templated (today's behavior)                     | literal           |
| `{@template name}` | literal         | templated                                        | templated         |
| `{@raw name}`      | literal         | literal                                          | literal           |

Rendering a `{@raw name}` provider never consults the minijinja engine; `[data]` namespaces it does not use are not loaded for it (a follow-up perf nicety).

## Rollout

1. **Patch:** the diagnostics/warning quick win above.
2. **Minor:** parser support for `{@raw …}` / `{@template …}` and `[data] default`; no behavior change for existing tags. Skill and docs updated.
3. **1.0 (optional):** flip the documented recommendation to explicit `{@template …}` for data-driven providers, keeping bare `{@name}` = today's semantics for compatibility. Deciding this now is not required — steps 1–2 stand alone.

## Open questions

- Should `{@raw}` also be expressible per _template file_ (a `.t.md` header comment) for repos where nearly every provider is prose? `[data] default = "raw"` may be enough; revisit after real usage.
- Does the `~` inline block need a matching form? No — inline blocks are templates by construction (`{~name:"{{ pkg.version }}"}`); their literal-brace problem is solved by `{{ "{{" }}` and is documented.
