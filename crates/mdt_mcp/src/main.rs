fn main() {
	let rt = tokio::runtime::Runtime::new().unwrap_or_else(|e| {
		eprintln!("mdt-mcp: failed to create runtime: {e}");
		std::process::exit(1);
	});
	rt.block_on(mdt_mcp::run_server());
}
