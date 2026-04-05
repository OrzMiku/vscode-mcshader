use super::*;

impl ServerCore {
    pub fn find_definitions(&self, params: GotoDefinitionParams) -> Option<Vec<Location>> {
        let server_data = self.server_data.lock().unwrap();
        let workspace_files = &server_data.workspace_files;
        let temp_files = &server_data.temp_files;

        let text_document_position = params.text_document_position_params;
        let file_path = text_document_position.text_document.uri.to_file_path().unwrap();

        let file: &dyn ShaderFile = if let Some(workspace_file) = workspace_files.get(&file_path) {
            workspace_file as &WorkspaceFile
        } else {
            temp_files.get(&file_path)?
        };

        if let Some(location) = file.include_definition(text_document_position.position) {
            return Some(vec![location]);
        }

        let content = file.content().borrow();
        let tree = file.tree().borrow();
        let line_mapping = file.line_mapping().borrow();

        TreeParser::find_definitions(
            &text_document_position.text_document.uri,
            text_document_position.position,
            &tree,
            &content,
            &line_mapping,
        )
    }
}
