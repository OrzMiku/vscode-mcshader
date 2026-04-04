use tower_lsp::{LspService, Server};
use vscode_mcshader::MinecraftLanguageServer;

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(MinecraftLanguageServer::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}
