//! Binary-crate tests. This file is named `__tests.rs` so the repository's
//! own `[exclude] patterns` skip the mdt tags embedded in these fixtures.

use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::time::Duration;

use clap::Parser as _;
use mdt_cli::MdtCli;

use super::OutputFormat;
use super::UpdateOutputFormat;
use super::run_check;
use super::run_update;
use super::watch_loop;

/// The watch loop announces itself, re-runs on change, reports a failed
/// re-run without stopping, and ends when the watcher goes away.
#[test]
fn watch_loop_reruns_and_survives_errors_until_the_watcher_drops() {
	let (tx, mut rx) = mpsc::channel();
	let calls = Arc::new(AtomicUsize::new(0));

	let counter = Arc::clone(&calls);
	let handle = std::thread::spawn(move || {
		watch_loop(&mut rx, "checking", move || {
			if counter.fetch_add(1, Ordering::SeqCst) == 0 {
				Err("first rerun fails".into())
			} else {
				Ok(())
			}
		});
	});

	// Spaced wider than the 200ms debounce, so each event is its own
	// re-run: the first hits the error path, the second the success path.
	tx.send(()).unwrap_or_else(|error| panic!("send: {error}"));
	std::thread::sleep(Duration::from_millis(400));
	tx.send(()).unwrap_or_else(|error| panic!("send: {error}"));
	std::thread::sleep(Duration::from_millis(400));
	drop(tx);

	handle
		.join()
		.unwrap_or_else(|_| panic!("watch thread panicked"));
	assert!(
		calls.load(Ordering::SeqCst) >= 2,
		"both reruns must have run"
	);
}

/// The watch entry points in `run_check` and `run_update` only execute in
/// watch mode, which never returns on its own. Run them on a detached
/// thread, give the thread time to reach the watch loop, and let the
/// process exit end it — the same lifecycle Ctrl+C gives a real user.
#[test]
fn check_watch_mode_enters_the_watch_loop() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@greeting} -->\n\nHi!\n\n<!-- {/greeting} -->\n",
	)?;
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=greeting} -->\n\nHi!\n\n<!-- {/greeting} -->\n",
	)?;
	let root = tmp.path().to_path_buf();
	let watch_root = root.clone();

	std::thread::spawn(move || {
		let args = MdtCli::parse_from([
			"mdt",
			"check",
			"--watch",
			"--no-color",
			"--path",
			watch_root.as_os_str().to_string_lossy().as_ref(),
		]);
		let _ = run_check(&args, false, OutputFormat::Text, true);
	});

	std::thread::sleep(Duration::from_millis(600));

	// A change fires the watcher so the re-run closure executes too.
	let _ = std::fs::write(
		root.join("readme.md"),
		"<!-- {=greeting} -->\n\nHi again!\n\n<!-- {/greeting} -->\n",
	);
	std::thread::sleep(Duration::from_millis(900));
	Ok(())
}

#[test]
fn update_watch_mode_enters_the_watch_loop() -> std::io::Result<()> {
	let tmp = tempfile::tempdir()?;
	std::fs::write(
		tmp.path().join("template.t.md"),
		"<!-- {@greeting} -->\n\nHi!\n\n<!-- {/greeting} -->\n",
	)?;
	std::fs::write(
		tmp.path().join("readme.md"),
		"<!-- {=greeting} -->\n\nHi!\n\n<!-- {/greeting} -->\n",
	)?;
	let root = tmp.path().to_path_buf();
	let watch_root = root.clone();

	std::thread::spawn(move || {
		let args = MdtCli::parse_from([
			"mdt",
			"update",
			"--watch",
			"--no-color",
			"--path",
			watch_root.as_os_str().to_string_lossy().as_ref(),
		]);
		let _ = run_update(&args, false, true, &[], &[], UpdateOutputFormat::Text);
	});

	std::thread::sleep(Duration::from_millis(600));

	// A change fires the watcher so the re-run closure executes too.
	let _ = std::fs::write(
		root.join("readme.md"),
		"<!-- {=greeting} -->\n\nHi again!\n\n<!-- {/greeting} -->\n",
	);
	std::thread::sleep(Duration::from_millis(900));
	Ok(())
}
