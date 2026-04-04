use super::*;

impl ServerCore {
    pub fn save_file(&self, url: Url) -> Option<Diagnostics> {
        let file_path = url.to_file_path().unwrap();

        let mut server_data = self.server_data.lock().unwrap();
        let temp_lint = server_data.temp_lint;
        let extensions = server_data.extensions.clone();
        let ServerData {
            tree_sitter_parser: parser,
            workspace_files,
            temp_files,
            ..
        } = &mut *server_data;

        let diagnostics = if let Some((file_path, workspace_file)) = workspace_files.get_key_value(&file_path) {
            // If this file is ended with watched extension, it should get updated through update_watched_files
            if file_path.extension().is_none_or(|ext| extensions.contains(ext.to_str().unwrap())) {
                return None;
            }
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
            self.collect_diagnostics(&update_list)
        } else {
            let temp_file = temp_files.get(&file_path)?;
            temp_file.update_from_disc(parser, &file_path);
            temp_file.parse_includes(&file_path);
            self.lint_temp_file(temp_file, &file_path, url, temp_lint)
        };

        self.collect_memory(workspace_files);
        Some(diagnostics)
    }
}
