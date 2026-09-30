//! The mdt agent skill, embedded so `mdt skill` works without the
//! `@m-d-t/skills` package installed and always matches this binary.
//!
//! The canonical files live in `packages/m-d-t__skills/skills/mdt/`. Cargo
//! only packages files inside the crate, so `mdt_cli/skill/` holds a copy
//! that the `skill` integration tests keep byte-identical to the canonical
//! files. After editing the skill, run `fix:skill` to refresh the copy.

use std::path::Path;
use std::path::PathBuf;

/// Directory name agents expect for this skill inside a skills directory.
pub(crate) const SKILL_DIR_NAME: &str = "mdt";

/// The skill entrypoint: frontmatter plus the core workflow.
pub(crate) const SKILL_MD: &str = include_str!("../skill/SKILL.md");

/// The detailed reference that `SKILL.md` points to.
pub(crate) const REFERENCE_MD: &str = include_str!("../skill/REFERENCE.md");

/// Every file in the skill, keyed by its file name.
const SKILL_FILES: [(&str, &str); 2] = [("SKILL.md", SKILL_MD), ("REFERENCE.md", REFERENCE_MD)];

/// Write the skill to `<skills_dir>/mdt/`, replacing any previous copy of its
/// files, and return the paths that were written.
pub(crate) fn install(skills_dir: &Path) -> std::io::Result<Vec<PathBuf>> {
	let target = skills_dir.join(SKILL_DIR_NAME);
	std::fs::create_dir_all(&target)?;

	SKILL_FILES
		.iter()
		.map(|(name, contents)| {
			let path = target.join(name);
			std::fs::write(&path, contents)?;
			Ok(path)
		})
		.collect()
}
