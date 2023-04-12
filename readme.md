# mdt

> manage **m**ark**d**own **t**emplates across your project

## Motivation

When managing larger libraries I find myself copying and pasting the examples across markdown and
code documentation.

These examples and explanations are often identical but need to be presented in separate places.

The problem, examples can quickly fall out of date and synchronizing them is painful.

This project provides a syntax to manage markdown templates `mdt` across all the files in your
project. It can also manage the templates inside code documentation for any language that supports
markdown in their documentation comments (which is most of them).

## Contributing

[`devenv`](https://devenv.sh/) is used to provide a reproducible development environment for this
project. Follow the [getting started instructions](https://devenv.sh/getting-started/).

To automatically load the environment you should
[install direnv](https://devenv.sh/automatic-shell-activation/) and then load the `direnv`.

```bash
# The security mechanism didn't allow to load the `.envrc`.
# Since we trust it, let's allow it execution.
direnv allow .
```

At this point you should see the `nix` commands available in your terminal.

To setup recommended configuration for your favourite editor run the following commands.

```bash
setup:vscode # Setup vscode
setup:helix  # Setup helix configuration
```
