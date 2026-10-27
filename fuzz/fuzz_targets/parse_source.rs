#![no_main]
//! Fuzz the source-comment path used for `.rs`, `.ts`, `.py`, and the other
//! scanned code extensions: comment-prefix stripping, fenced code-block
//! filtering, and block parsing inside comments.

use libfuzzer_sys::fuzz_target;
use mdt_core::CodeBlockFilter;

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

	let _ = mdt_core::parse_source_with_diagnostics(data, &CodeBlockFilter::default());
});
