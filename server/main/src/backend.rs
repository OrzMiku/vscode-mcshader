use logging::{info, warn};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use crate::progress::ProgressReporter;
use crate::worker::ServerWorker;

pub struct MinecraftLanguageServer {
    client: Client,
    worker: ServerWorker,
    progress: ProgressReporter,
    _log_guard: logging::GlobalLoggerGuard,
}

impl MinecraftLanguageServer {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            worker: ServerWorker::new(),
            progress: ProgressReporter::new(),
            _log_guard: logging::init_logger(),
        }
    }

    async fn publish_diagnostics(&self, diagnostics: crate::server::Diagnostics) {
        for (uri, diagnostics) in diagnostics {
            self.client.publish_diagnostics(uri, diagnostics, None).await;
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for MinecraftLanguageServer {
    #[logging::with_trace_id]
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        self.progress.set_enabled(
            params
                .capabilities
                .window
                .as_ref()
                .and_then(|window| window.work_done_progress)
                .unwrap_or(false),
        );

        self.worker.request(move |core| core.initialize(params)).await
    }

    #[logging::with_trace_id]
    async fn initialized(&self, params: InitializedParams) {
        self.worker.request(move |core| core.initialized(params)).await;
        info!("Language server initialized");
    }

    async fn shutdown(&self) -> Result<()> {
        self.worker.request(|core| core.shutdown()).await
    }

    #[logging::with_trace_id]
    async fn execute_command(&self, params: ExecuteCommandParams) -> Result<Option<serde_json::Value>> {
        self.worker.request(move |core| core.execute_command(params)).await
    }

    #[logging::with_trace_id]
    async fn did_change_configuration(&self, params: DidChangeConfigurationParams) {
        let registrations = self.worker.request(move |core| core.did_change_configuration(params)).await;
        if let Err(err) = self.client.register_capability(registrations).await {
            warn!("Unable to register file watch capability: {}", err);
        }
    }

    #[logging::with_trace_id]
    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        if let Some(diagnostics) = self.worker.request(move |core| core.open_file(params)).await {
            self.publish_diagnostics(diagnostics).await;
        }
    }

    #[logging::with_trace_id]
    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        if let Some(diagnostics) = self.worker.request(move |core| core.change_file(uri, params.content_changes)).await {
            self.publish_diagnostics(diagnostics).await;
        }
    }

    #[logging::with_trace_id]
    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        if let Some(diagnostics) = self.worker.request(move |core| core.save_file(params.text_document.uri)).await {
            self.publish_diagnostics(diagnostics).await;
        }
    }

    #[logging::with_trace_id]
    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        if let Some(diagnostics) = self.worker.request(move |core| core.close_file(params.text_document.uri)).await {
            self.publish_diagnostics(diagnostics).await;
        }
    }

    #[logging::with_trace_id]
    async fn will_rename_files(&self, params: RenameFilesParams) -> Result<Option<WorkspaceEdit>> {
        self.worker.request(move |core| core.will_rename_files(params)).await
    }

    #[logging::with_trace_id]
    async fn document_link(&self, params: DocumentLinkParams) -> Result<Option<Vec<DocumentLink>>> {
        match self.worker.request(move |core| core.document_links(params.text_document.uri)).await {
            Some((links, diagnostics)) => {
                self.publish_diagnostics(diagnostics).await;
                Ok(Some(links))
            }
            None => Ok(None),
        }
    }

    #[logging::with_trace_id]
    async fn goto_definition(&self, params: GotoDefinitionParams) -> Result<Option<GotoDefinitionResponse>> {
        self.worker.request(move |core| core.goto_definition(params)).await
    }

    #[logging::with_trace_id]
    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        self.worker.request(move |core| core.hover(params)).await
    }

    #[logging::with_trace_id]
    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        self.worker.request(move |core| core.references(params)).await
    }

    #[logging::with_trace_id]
    async fn document_symbol(&self, params: DocumentSymbolParams) -> Result<Option<DocumentSymbolResponse>> {
        self.worker.request(move |core| core.document_symbol(params)).await
    }

    #[logging::with_trace_id]
    async fn did_change_workspace_folders(&self, params: DidChangeWorkspaceFoldersParams) {
        let diagnostics = self
            .progress
            .run(
                &self.client,
                "Refreshing workspace",
                Some("Re-indexing shader packs".to_owned()),
                self.worker.request(move |core| core.update_workspaces(params.event)),
            )
            .await;

        self.publish_diagnostics(diagnostics).await;
    }

    #[logging::with_trace_id]
    async fn did_change_watched_files(&self, params: DidChangeWatchedFilesParams) {
        let diagnostics = self
            .progress
            .run(
                &self.client,
                "Refreshing files",
                Some("Applying filesystem updates".to_owned()),
                self.worker.request(move |core| core.update_watched_files(&params.changes)),
            )
            .await;

        self.publish_diagnostics(diagnostics).await;
    }
}
