#[tokio::main]
#[allow(clippy::disallowed_methods)]
async fn main() {
	mdt_lsp::run_server().await;
}
