# Quality rules

## Formatting

- Use `fix:format` or `dprint fmt`.
- Do not run `rustfmt` directly.
- Follow existing formatting output instead of hand-formatting around it.

## Lint and safety constraints

- `unsafe_code` is denied workspace-wide.
- `unstable_features` is denied workspace-wide.
- `clippy::correctness` is denied.
- `clippy::wildcard_dependencies` is denied.
- `Result::expect` is disallowed; use `unwrap_or_else` with an explicit panic message instead.

## Bug-fix protocol

When fixing a logic bug:

1. reproduce it with a failing test first
2. verify the test fails for the intended reason
3. implement the fix
4. verify the new test passes
5. run the relevant broader test suite

## Testing guidance

- Use focused unit tests for isolated behavior.
- Use realistic fixtures for integration tests.
- Cover edge cases and error paths when they are relevant to the change.
- Prefer adding or updating the smallest test set that proves the behavior.
