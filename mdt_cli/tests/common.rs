use assert_cmd::Command;
use insta_cmd::get_cargo_bin;

pub fn mdt_cmd() -> Command {
	let mut cmd = Command::new(get_cargo_bin("mdt"));
	cmd.env("NO_COLOR", "1");
	cmd
}
