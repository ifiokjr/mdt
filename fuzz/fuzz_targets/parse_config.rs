#![no_main]
//! Fuzz `mdt.toml` deserialization: arbitrary TOML must produce a config
//! error or a valid config, never a panic or a hang in the deserializer.

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

	let _ = toml::from_str::<mdt_core::MdtConfig>(data);
});
