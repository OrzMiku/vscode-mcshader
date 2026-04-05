use super::*;

#[must_use]
fn function_ref_pattern(name: &str) -> String {
    r#"((call_expression(identifier) @call)(#match? @call "^"#.to_owned() + name + r#"$"))"#
}

#[must_use]
fn variable_def_pattern(name: &str) -> String {
    let mut pattern = r#"[
        (init_declarator
            declarator: (identifier) @variable)

        (parameter_declaration
            declarator: (identifier) @variable)

        (declaration
            declarator: (identifier) @variable)

        (preproc_def
            name: (identifier) @variable)

        (#match? @variable "^"#
        .to_owned();
    pattern += name;
    pattern += r#"$")]"#;
    pattern
}

#[must_use]
fn variable_ref_pattern(name: &str) -> String {
    r#"((identifier) @variable_ref (#match? @variable_ref "^"#.to_owned() + name + r#"$"))"#
}

impl TreeParser {
    fn resolve_variable_definition<'a>(content: &str, start_node: Node<'a>) -> Option<Node<'a>> {
        let query_str = variable_def_pattern(start_node.utf8_text(content.as_bytes()).ok()?);
        let query = Query::new(&tree_sitter_glsl::LANGUAGE_GLSL.into(), &query_str).ok()?;
        let mut query_cursor = QueryCursor::new();
        query_cursor.set_byte_range(0..start_node.end_byte());

        let mut parent = Some(start_node);
        while let Some(parent_node) = parent {
            let mut latest_match = None;

            for query_match in query_cursor.matches(&query, parent_node, content.as_bytes()) {
                latest_match = query_match.captures.last().map(|capture| capture.node);
            }

            if latest_match.is_some() {
                return latest_match;
            }

            parent = parent_node.parent();
        }

        None
    }

    fn variable_reference_scope(definition: Node<'_>) -> Node<'_> {
        let mut parent = definition.parent();

        while let Some(parent_node) = parent {
            match parent_node.kind() {
                "compound_statement" | "function_definition" | "preproc_function_def" | "translation_unit" => return parent_node,
                _ => parent = parent_node.parent(),
            }
        }

        definition
    }

    fn find_variable_references(url: &Url, content: &str, definition: Node<'_>, line_mapping: &[usize]) -> Vec<Location> {
        let query_str = variable_ref_pattern(definition.utf8_text(content.as_bytes()).unwrap());
        let query = Query::new(&tree_sitter_glsl::LANGUAGE_GLSL.into(), &query_str).unwrap();
        let mut query_cursor = QueryCursor::new();

        let scope = Self::variable_reference_scope(definition);
        let definition_range = definition.range();
        let mut locations = vec![];

        for query_match in query_cursor.matches(&query, scope, content.as_bytes()) {
            for capture in query_match.captures {
                let node = capture.node;
                let Some(resolved_definition) = Self::resolve_variable_definition(content, node) else {
                    continue;
                };

                if resolved_definition.range() == definition_range {
                    locations.push(node.to_location(url, content, line_mapping));
                }
            }
        }

        locations
    }

    pub fn find_references(url: &Url, position: Position, tree: &Tree, content: &str, line_mapping: &[usize]) -> Option<Vec<Location>> {
        let current_node = Self::current_node_fetch(position, tree, content, line_mapping)?;
        let parent = current_node.parent()?;

        match (current_node.kind(), parent.kind()) {
            (_, "function_declarator" | "preproc_function_def") => {
                let query_str = function_ref_pattern(current_node.utf8_text(content.as_bytes()).unwrap());
                Some(Self::simple_global_search(url, tree, content, &query_str, line_mapping))
            }
            ("identifier", "parameter_declaration" | "declaration" | "init_declarator" | "preproc_def") => {
                let definition = match parent.kind() {
                    "init_declarator" if current_node.prev_sibling().is_some() => Self::resolve_variable_definition(content, current_node)?,
                    _ => current_node,
                };
                Some(Self::find_variable_references(url, content, definition, line_mapping))
            }
            ("identifier", "argument_list" | "field_expression" | "binary_expression" | "return_statement" | "assignment_expression") => {
                let definition = Self::resolve_variable_definition(content, current_node)?;
                Some(Self::find_variable_references(url, content, definition, line_mapping))
            }
            _ => None,
        }
    }
}
