use super::*;

fn hover_label(node: Node<'_>, parent: Node<'_>) -> Option<&'static str> {
    match (node.kind(), parent.kind()) {
        (_, "call_expression" | "function_declarator" | "preproc_function_def") => Some("Function"),
        ("identifier", "parameter_declaration" | "declaration" | "init_declarator" | "preproc_def") => Some("Variable"),
        ("field_identifier", _) => Some("Field"),
        ("type_identifier", "struct_specifier") => Some("Struct"),
        ("identifier", _) => Some("Identifier"),
        _ => None,
    }
}

impl TreeParser {
    pub fn hover(position: Position, tree: &Tree, content: &str, line_mapping: &[usize]) -> Option<Hover> {
        let current_node = Self::current_node_fetch(position, tree, content, line_mapping)?;
        let parent = current_node.parent()?;
        let label = hover_label(current_node, parent)?;
        let snippet_node = match parent.kind() {
            "call_expression" => current_node,
            _ => parent,
        };
        let snippet = snippet_node.utf8_text(content.as_bytes()).ok()?.trim();
        let snippet = if snippet.is_empty() {
            current_node.utf8_text(content.as_bytes()).ok()?
        } else {
            snippet
        };

        Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: format!("**{}**\n```glsl\n{}\n```", label, snippet),
            }),
            range: Some(current_node.to_range(content, line_mapping)),
        })
    }
}
