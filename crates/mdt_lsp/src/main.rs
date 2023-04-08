use mdt_lsp::MdtLanguageServer;
use tower_lsp::LspService;
use tower_lsp::Server;

#[tokio::main]
async fn main() {
  let stdin = tokio::io::stdin();
  let stdout = tokio::io::stdout();

  let (service, socket) = LspService::new(MdtLanguageServer::new);
  Server::new(stdin, stdout, socket).serve(service).await;
}
