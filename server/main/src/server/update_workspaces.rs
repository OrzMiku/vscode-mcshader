use super::*;

impl ServerCore {
    pub fn update_workspaces(&self, events: WorkspaceFoldersChangeEvent) -> Diagnostics {
        let mut server_data = self.server_data.lock().unwrap();
        let ServerData {
            tree_sitter_parser: parser,
            shader_packs,
            workspace_files,
            temp_files,
            ..
        } = &mut *server_data;

        let mut diagnostics: Diagnostics = HashMap::new();
        for removed_workspace in &events.removed {
            let removed_path = removed_workspace.uri.to_file_path().unwrap();
            let removed_shader_packs: HashSet<_> = shader_packs
                .extract_if(|pack_path| pack_path.path.starts_with(&removed_path))
                .collect();
            diagnostics.extend(
                workspace_files
                    .extract_if(|_, workspace_file| removed_shader_packs.contains(workspace_file.shader_pack()))
                    .map(|(file_path, _)| (Url::from_file_path(&file_path as &Path).unwrap(), vec![])),
            );
        }

        for added_workspace in events.added {
            let added_path = added_workspace.uri.to_file_path().unwrap();
            self.scan_files_in_root(parser, shader_packs, workspace_files, temp_files, added_path);
        }
        diagnostics
    }
}
