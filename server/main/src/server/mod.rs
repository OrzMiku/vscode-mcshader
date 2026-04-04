#![allow(deprecated)]

use std::path::{MAIN_SEPARATOR, Path, PathBuf};
use std::str::FromStr;
use std::sync::Mutex;

use logging::{error, info};

use hashbrown::{HashMap, HashSet};
use serde_json::Value;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tree_sitter::Parser;

mod change_file;
mod close_file;
mod document_links;
mod error;
mod find_definitions;
mod find_references;
mod hover;
mod list_symbols;
mod open_file;
mod rename_files;
mod save_file;
mod update_watched_files;
mod update_workspaces;
mod utility;

use crate::capability::ServerCapabilitiesFactroy;
use crate::configuration::Configuration;
use crate::constant::*;
use crate::file::*;
use crate::tree_parser::TreeParser;

pub type Diagnostics = HashMap<Url, Vec<Diagnostic>>;

/// Everything mutable in this struct.
/// By sending the Mutex of server data to snyc functions, we can handle it like single thread
pub struct ServerData {
    pub temp_lint: bool,
    pub extensions: HashSet<Box<str>>,
    pub shader_packs: HashSet<std::rc::Rc<ShaderPack>>,
    pub workspace_files: HashMap<std::rc::Rc<PathBuf>, std::rc::Rc<WorkspaceFile>>,
    pub temp_files: HashMap<PathBuf, TempFile>,
    pub tree_sitter_parser: Parser,
}

impl ServerData {
    pub fn new() -> Self {
        let mut tree_sitter_parser = Parser::new();
        tree_sitter_parser.set_language(&tree_sitter_glsl::LANGUAGE_GLSL.into()).unwrap();
        Self {
            temp_lint: false,
            extensions: BASIC_EXTENSIONS.clone(),
            shader_packs: HashSet::new(),
            workspace_files: HashMap::new(),
            temp_files: HashMap::new(),
            tree_sitter_parser,
        }
    }
}

pub struct ServerCore {
    server_data: Mutex<ServerData>,
}

pub struct LanguageServerError;

impl ServerCore {
    pub fn new() -> Self {
        Self {
            server_data: Mutex::new(ServerData::new()),
        }
    }

    #[logging::with_trace_id]
    pub fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        info!("Starting server...");

        assert!(
            params
                .capabilities
                .general
                .unwrap()
                .position_encodings
                .is_none_or(|encs| encs.is_empty() || encs.contains(&PositionEncodingKind::UTF16))
        );

        let mut initialize_result = ServerCapabilitiesFactroy::initial_capabilities();
        initialize_result.capabilities.position_encoding = Some(PositionEncodingKind::UTF16);

        let roots: Vec<PathBuf> = if let Some(workspaces) = params.workspace_folders {
            workspaces
                .iter()
                .map(|workspace| workspace.uri.to_file_path().unwrap())
                .collect::<Vec<_>>()
        } else if let Some(uri) = params.root_uri {
            vec![uri.to_file_path().unwrap(); 1]
        } else {
            vec![]
        };

        self.initial_scan(roots);

        Ok(initialize_result)
    }

    pub const fn initialized(&self, _params: InitializedParams) {}

    pub const fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    #[logging::with_trace_id]
    pub fn execute_command(&self, params: ExecuteCommandParams) -> Result<Option<Value>> {
        let server_data = self.server_data.lock().unwrap();
        match COMMAND_LIST.get(params.command.as_str()) {
            Some(command) => command.run(&params.arguments, &server_data),
            None => Err(LanguageServerError::invalid_command_error()),
        }
    }

    #[logging::with_trace_id]
    pub fn did_change_configuration(&self, params: DidChangeConfigurationParams) -> Vec<Registration> {
        info!("Got updated configuration"; "config" => params.settings.as_object().unwrap().get("mcshader").unwrap().to_string());

        let mut config = Configuration::new(&params.settings);

        let registrations = config.generate_file_watch_registration();

        match logging::Level::from_str(&config.log_level) {
            Ok(level) => logging::set_level(level),
            Err(()) => error!("Got unexpected log level from config"; "level" => &config.log_level),
        }

        config.extra_extension.extend(BASIC_EXTENSIONS.clone());

        let mut server_data = self.server_data.lock().unwrap();
        server_data.extensions = config.extra_extension;
        server_data.temp_lint = config.temp_lint;
        registrations
    }

    pub fn will_rename_files(&self, params: RenameFilesParams) -> Result<Option<WorkspaceEdit>> {
        Ok(Some(self.rename_files(params)))
    }

    pub fn goto_definition(&self, params: GotoDefinitionParams) -> Result<Option<GotoDefinitionResponse>> {
        Ok(self.find_definitions(params).map(GotoDefinitionResponse::Array))
    }

    pub fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        Ok(self.find_references(params))
    }

    pub fn document_symbol(&self, params: DocumentSymbolParams) -> Result<Option<DocumentSymbolResponse>> {
        Ok(self.list_symbols(params))
    }

    pub fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        Ok(self.hover_info(params))
    }
}
