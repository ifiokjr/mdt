use std::path::Path;

use assert_cmd::Command;
use insta_cmd::get_cargo_bin;

pub fn mdt_cmd() -> Command {
	let mut cmd = Command::new(get_cargo_bin("mdt"));
	cmd.env("NO_COLOR", "1");
	cmd
}

/// Copy a named fixture directory into `dest`, preserving directory structure.
pub fn copy_fixture(name: &str, dest: &Path) {
	let src = Path::new(env!("CARGO_MANIFEST_DIR"))
		.join("tests/fixtures")
		.join(name);
	copy_dir_recursive(&src, dest);
}

fn copy_dir_recursive(src: &Path, dst: &Path) {
	std::fs::create_dir_all(dst)
		.unwrap_or_else(|e| panic!("create_dir_all {}: {e}", dst.display()));
	for entry in
		std::fs::read_dir(src).unwrap_or_else(|e| panic!("read_dir {}: {e}", src.display()))
	{
		let entry = entry.unwrap_or_else(|e| panic!("entry: {e}"));
		let src_path = entry.path();
		let dst_path = dst.join(entry.file_name());
		if src_path.is_dir() {
			copy_dir_recursive(&src_path, &dst_path);
		} else {
			std::fs::copy(&src_path, &dst_path).unwrap_or_else(|e| {
				panic!("copy {} -> {}: {e}", src_path.display(), dst_path.display())
			});
		}
	}
}
