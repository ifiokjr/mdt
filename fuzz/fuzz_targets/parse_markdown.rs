#![no_main]
//! Fuzz the markdown path: HTML-comment extraction, lexing, block parsing,
//! and content extraction between tags. This is the parser that runs on
//! every scanned `.md` file, including files from cloned repositories and
//! pull requests.

use libfuzzer_sys::fuzz_target;

// libFuzzer's panic hook aborts before `catch_unwind` boundaries in
// mdt_core can contain a known third-party panic; drop the hook so those
// convert into errors. A real, uncontained panic still unwinds out of this
// callback and aborts, failing the run.
static REMOVE_ABORT_HOOK: std::sync::Once = std::sync::Once::new();

fn remove_abort_hook() {
	REMOVE_ABORT_HOOK.call_once(|| {
		let _ = std::panic::take_hook();
	});
}

fuzz_target!(|data: &str| {
	remove_abort_hook();

	if let Ok((blocks, _)) = mdt_core::parse_with_diagnostics(data) {
		for block in &blocks {
			let _ = mdt_core::extract_content_between_tags(data, block);
		}
	}
});
