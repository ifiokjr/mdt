---
mdt_core: major
---

Expand config and data-source capabilities in `mdt_core`:

- Add config discovery precedence across `mdt.toml`, `.mdt.toml`, and `.config/mdt.toml`.
- Add typed `[data]` entries (`{ path, format }`) while keeping string-path compatibility.
- Add `ini` data format support.
- Expose new config/data APIs (`CONFIG_FILE_CANDIDATES`, `MdtConfig::resolve_path`, `DataSource`, `TypedDataSource`).

This is marked major because `MdtConfig.data` changes type from `HashMap<String, PathBuf>` to `HashMap<String, DataSource>`.
