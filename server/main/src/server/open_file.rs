use super::*;

impl ServerCore {
    pub fn open_file(&self, params: DidOpenTextDocumentParams) -> Option<Diagnostics> {
        let file_path = params.text_document.uri.to_file_path().unwrap();
        let file_url = params.text_document.uri.clone();

        let mut server_data = self.server_data.lock().unwrap();
        let temp_lint = server_data.temp_lint;
        let ServerData {
            tree_sitter_parser: parser,
            workspace_files,
            temp_files,
            ..
        } = &mut *server_data;

        if let Some((file_path, workspace_file)) = workspace_files.get_key_value(&file_path) {
            let content = params.text_document.text;
            *workspace_file.tree().borrow_mut() = parser.parse(&content, None).unwrap();
            *workspace_file.line_mapping().borrow_mut() = generate_line_mapping(&content);
            *workspace_file.content().borrow_mut() = content;

            // Clone the content so they can be used alone.
            let file_path = file_path.clone();
            let workspace_file = workspace_file.clone();
            let mut update_list = HashMap::new();

            WorkspaceFile::parse_content(
                workspace_files,
                temp_files,
                parser,
                &mut update_list,
                &workspace_file,
                &file_path,
                1,
            );
            let shader_files = workspace_file.parent_shaders().borrow();
            shader_files.iter().for_each(|(shader_path, shader_file)| {
                self.lint_workspace_shader(shader_file, shader_path, &mut update_list);
            });

            self.collect_memory(workspace_files);
            Some(self.collect_diagnostics(&update_list))
        } else {
            let temp_file = TempFile::new(parser, &file_path, params.text_document.text);
            temp_files.insert(file_path, temp_file);
            self.collect_memory(workspace_files);
            temp_files
                .get(&file_url.to_file_path().unwrap())
                .map(|temp_file| self.lint_temp_file(temp_file, &file_url.to_file_path().unwrap(), file_url, temp_lint))
        }
    }
}
