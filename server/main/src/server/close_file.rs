use super::*;

impl ServerCore {
    pub fn close_file(&self, file_url: Url) -> Option<Diagnostics> {
        let file_path = file_url.to_file_path().unwrap();

        let mut server_data = self.server_data.lock().unwrap();
        let ServerData {
            tree_sitter_parser: parser,
            workspace_files,
            temp_files,
            ..
        } = &mut *server_data;

        // Force closing may result in temp changes discarded, so the content should reset to the disc copy.
        let diagnostics = if let Some((file_path, workspace_file)) = workspace_files.get_key_value(&file_path) {
            workspace_file.update_from_disc(parser, file_path);
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

            let diagnostics = self.collect_diagnostics(&update_list);
            Some(diagnostics)
        } else {
            temp_files.remove(&file_path).map(|_| HashMap::from([(file_url, vec![])]))
        };

        self.collect_memory(workspace_files);
        diagnostics
    }
}
