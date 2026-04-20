#![allow(dead_code)]

use std::path::Path;
use std::process::Command as StdCommand;

use assert_cmd::Command;
use insta_cmd::get_cargo_bin;

pub fn mdt_cmd() -> Command {
	let mut cmd = Command::new(get_cargo_bin("mdt"));
	cmd.env("NO_COLOR", "1");
	cmd
}

pub fn mdt_std_cmd() -> StdCommand {
	let mut cmd = StdCommand::new(get_cargo_bin("mdt"));
	cmd.env("NO_COLOR", "1");
	cmd
}

pub fn mdt_cmd_for_path(path: &Path) -> StdCommand {
	let mut cmd = mdt_std_cmd();
	cmd.arg("--path");
	cmd.arg(path);
	cmd
}

pub fn with_redacted_temp_dir(tmp_path: &Path, f: impl FnOnce()) {
	let path_str = tmp_path.display().to_string();
	let mut escaped = String::with_capacity(path_str.len() * 2);

	for ch in path_str.chars() {
		if matches!(
			ch,
			'\\' | '.' | '+' | '*' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '^' | '$' | '|'
		) {
			escaped.push('\\');
		}
		escaped.push(ch);
	}

	let mut settings = insta::Settings::clone_current();
	settings.add_filter(&escaped, "[TEMP_DIR]");
	settings.add_filter(
		r#""timestamp_unix_ms": \d+"#,
		r#""timestamp_unix_ms": [UNIX_MS]"#,
	);
	settings.add_filter(
		r"Last scan unix ms\s+\d+",
		"Last scan unix ms            [UNIX_MS]",
	);
	settings.bind(f);
}

pub fn snapshot_path_id(path: &str) -> String {
	path.replace(['/', '.'], "_")
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
