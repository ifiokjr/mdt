mod common;

#[test]
fn completions_are_generated_for_every_shell() -> std::io::Result<()> {
	let shells = [
		("bash", "_mdt"),
		("zsh", "#compdef mdt"),
		("fish", "complete -c mdt"),
		("powershell", "Register-ArgumentCompleter"),
		("elvish", "edit:completion"),
	];

	for (shell, marker) in shells {
		let output = common::mdt_cmd().arg("completions").arg(shell).output()?;

		assert!(
			output.status.success(),
			"{shell} completions failed: {}",
			String::from_utf8_lossy(&output.stderr)
		);

		let stdout = String::from_utf8_lossy(&output.stdout);
		assert!(!stdout.trim().is_empty(), "{shell} completions are empty");
		assert!(
			stdout.contains(marker),
			"{shell} completions do not contain `{marker}`:\n{stdout}"
		);
	}

	Ok(())
}

#[test]
fn completions_reject_unknown_shells() -> std::io::Result<()> {
	let output = common::mdt_cmd().arg("completions").arg("tcsh").output()?;

	assert_eq!(output.status.code(), Some(2));

	Ok(())
}
