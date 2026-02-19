# Quick Start

This walkthrough creates a small project that uses mdt to keep a README in sync with a template.

## 1. Initialize a project

Create a new directory and generate a starter template:

```sh
mkdir my-project && cd my-project
mdt init
```

This creates `template.t.md` with a sample provider block:

```markdown
<!-- {@greeting} -->

Hello from mdt! This is a provider block.

<!-- {/greeting} -->
```

## 2. Add a consumer

Create a `readme.md` that references the provider:

```markdown
# My Project

Welcome to my project.

<!-- {=greeting} -->

This will be replaced by mdt.

<!-- {/greeting} -->
```

The `{=greeting}` tag marks this as a **consumer** of the `greeting` provider. The content between the opening and closing tags will be replaced.

## 3. Update

Run the update command:

```sh
mdt update
```

Output:

```
Updated 1 block(s) in 1 file(s).
```

Now `readme.md` contains:

```markdown
# My Project

Welcome to my project.

<!-- {=greeting} -->

Hello from mdt! This is a provider block.

<!-- {/greeting} -->
```

The content between the consumer tags has been replaced with the provider's content.

## 4. Check for staleness

Edit the provider in `template.t.md`:

```markdown
<!-- {@greeting} -->

Hello from mdt! This content has been updated.

<!-- {/greeting} -->
```

Now run the check command:

```sh
mdt check
```

Output:

```
Stale: block `greeting` in readme.md

1 consumer block(s) are out of date. Run `mdt update` to fix.
```

The check command exits with a non-zero status code when blocks are stale, making it useful in CI pipelines.

## 5. See what changed

Use the `--diff` flag to see exactly what's different:

```sh
mdt check --diff
```

This shows a colorized unified diff between the current consumer content and what the provider would produce.

## 6. List all blocks

See all providers and consumers in the project:

```sh
mdt list
```

Output:

```
Providers:
  @greeting template.t.md (1 consumer(s))

Consumers:
  =greeting readme.md [linked]

1 provider(s), 1 consumer(s)
```

## Next steps

- Learn about [providers and consumers](../concepts/providers-and-consumers.md) in depth
- Add [data interpolation](../guide/data-interpolation.md) to pull values from project files
- Use [transformers](../guide/transformers.md) to adapt content for different contexts
- Set up [CI integration](../guide/ci-integration.md) to catch stale docs automatically
