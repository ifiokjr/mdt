---
mdt_core: none
---

# Polish internal core code for clippy

Internal core implementation details were adjusted to satisfy clippy without changing public behavior. The cleanup renames confusing local variables, uses `std::io::Error::other()` where appropriate, and narrows an `unnecessary_wraps` allowance to helper code that intentionally preserves a shared signature.

These changes reduce lint noise and keep future warnings focused on meaningful regressions.
