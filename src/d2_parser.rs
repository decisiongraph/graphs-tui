//! D2 diagram language parser
//!
//! D2 syntax:
//! - Shapes: `id` or `id: "Label"`
//! - Connections: `->`, `<-`, `<->`, `--`
//! - Connection chains: `A -> B -> C`
//! - Shape types: `id.shape: circle`
//! - Containers: `parent { child }` (multi-level nesting)
//! - Nested keys: `a.b.c: "Label"`
//! - Edge labels: `A -> B: "label"`
//! - SQL tables/classes with fields
//! - Quoted keys: `"my node" -> "other node"`
//! - Semicolons: `A -> B; C -> D`
//! - Null deletion: `x: null`

use winnow::ascii::{space0, Caseless};
use winnow::combinator::alt;
use winnow::error::{ErrMode, ParserError};
use winnow::token::{rest, take_until};
use winnow::ModalResult;
use winnow::Parser;

use crate::error::MermaidError;
use crate::types::{
    DiagramWarning, Direction, Edge, EdgeStyle, Graph, Node, NodeId, NodeShape, Subgraph,
    TableField,
};

// ===== Winnow parsers =====

/// Parse direction declaration: "direction: right|left|down|up"
fn w_direction(input: &mut &str) -> ModalResult<Direction> {
    let _ = "direction:".parse_next(input)?;
    let _ = space0.parse_next(input)?;
    alt((
        Caseless("right").value(Direction::LR),
        Caseless("left").value(Direction::RL),
        Caseless("down").value(Direction::TB),
        Caseless("up").value(Direction::BT),
    ))
    .parse_next(input)
}

/// Parse shape property: "id.shape: type"
fn w_shape_property(input: &mut &str) -> ModalResult<(String, NodeShape)> {
    let id: &str = take_until(1.., ".shape:").parse_next(input)?;
    let _ = ".shape:".parse_next(input)?;
    let _ = space0.parse_next(input)?;
    let shape_str: &str = rest.parse_next(input)?;
    let shape = parse_shape_str(&shape_str.trim().to_lowercase());
    Ok((id.trim().to_string(), shape))
}

/// Parse label property: "id.label: text"
fn w_label_property(input: &mut &str) -> ModalResult<(String, String)> {
    let id: &str = take_until(1.., ".label:").parse_next(input)?;
    let _ = ".label:".parse_next(input)?;
    let _ = space0.parse_next(input)?;
    let label: &str = rest.parse_next(input)?;
    let label = label
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_string();
    Ok((id.trim().to_string(), label))
}

/// Parse standalone shape inside container: "shape: type"
fn w_standalone_shape(input: &mut &str) -> ModalResult<NodeShape> {
    let _ = "shape:".parse_next(input)?;
    let _ = space0.parse_next(input)?;
    let shape_str: &str = rest.parse_next(input)?;
    Ok(parse_shape_str(&shape_str.trim().to_lowercase()))
}

/// Parse table field with optional type and constraint
fn w_table_field(input: &mut &str) -> ModalResult<TableField> {
    let line: &str = rest.parse_next(input)?;
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return Err(ErrMode::from_input(input));
    }

    let (main_part, constraint) = if let Some(brace_start) = line.find('{') {
        let main = line[..brace_start].trim();
        let brace_content = line[brace_start + 1..].trim_end_matches('}').trim();
        let constraint = if let Some(stripped) = brace_content.strip_prefix("constraint:") {
            Some(stripped.trim().to_string())
        } else {
            Some(brace_content.to_string())
        };
        (main, constraint)
    } else {
        (line, None)
    };

    if let Some(colon_idx) = main_part.find(':') {
        let name = main_part[..colon_idx].trim().to_string();
        let type_info = main_part[colon_idx + 1..].trim().to_string();
        Ok(TableField {
            name,
            type_info: if type_info.is_empty() {
                None
            } else {
                Some(type_info)
            },
            constraint,
        })
    } else {
        Ok(TableField {
            name: main_part.to_string(),
            type_info: None,
            constraint: None,
        })
    }
}

/// Result of parsing D2: a graph plus any warnings
pub struct D2ParseResult {
    pub graph: Graph,
    pub warnings: Vec<DiagramWarning>,
}

/// Parse D2 diagram syntax into a Graph
pub fn parse_d2(input: &str) -> Result<D2ParseResult, MermaidError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(MermaidError::EmptyInput);
    }

    let mut graph = Graph::new(Direction::TB);
    let mut warnings: Vec<DiagramWarning> = Vec::new();
    let mut container_stack: Vec<String> = Vec::new();
    let mut table_nodes: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut null_nodes: Vec<String> = Vec::new();

    for (line_idx, raw_line) in trimmed.lines().enumerate() {
        let line_num = line_idx + 1;
        let line = raw_line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Handle closing braces
        if line == "}" || (line.starts_with('}') && !line.contains('{')) {
            let closing_count = line.chars().filter(|&c| c == '}').count();
            for _ in 0..closing_count {
                container_stack.pop();
            }
            continue;
        }

        // Split on semicolons
        let segments: Vec<&str> = split_on_semicolons(line);

        for segment in segments {
            let segment = segment.trim();
            if segment.is_empty() {
                continue;
            }

            process_segment(
                segment,
                line_num,
                &mut graph,
                &mut warnings,
                &mut container_stack,
                &mut table_nodes,
                &mut null_nodes,
            );
        }
    }

    // Remove null-deleted nodes
    for id in &null_nodes {
        graph.nodes.remove(id);
        graph.edges.retain(|e| e.from != *id && e.to != *id);
    }

    if graph.nodes.is_empty() && graph.edges.is_empty() {
        return Err(MermaidError::ParseError {
            line: 1,
            message: "No valid D2 content found".to_string(),
            suggestion: Some(
                "D2 syntax: 'A -> B' for connections, 'name: Label' for nodes".to_string(),
            ),
        });
    }

    Ok(D2ParseResult { graph, warnings })
}

fn process_segment(
    segment: &str,
    line_num: usize,
    graph: &mut Graph,
    warnings: &mut Vec<DiagramWarning>,
    container_stack: &mut Vec<String>,
    table_nodes: &mut std::collections::HashSet<String>,
    null_nodes: &mut Vec<String>,
) {
    let current_subgraph = container_stack.last().cloned();

    // Direction at root level
    if container_stack.is_empty() {
        let mut input = segment;
        if let Ok(dir) = w_direction(&mut input) {
            graph.direction = dir;
            return;
        }
    }

    // Check unsupported features
    if check_unsupported(segment, line_num, warnings) {
        return;
    }

    // Style properties
    if is_style_property(segment) {
        return;
    }

    // Container open
    if segment.ends_with('{') {
        let container_def = segment.trim_end_matches('{').trim();
        if !container_def.is_empty() {
            handle_container_open(container_def, graph, container_stack, table_nodes);
        }
        return;
    }

    // Standalone shape: inside container
    if !container_stack.is_empty() {
        let mut input = segment;
        if let Ok(shape) = w_standalone_shape(&mut input) {
            if let Some(container_id) = container_stack.last() {
                if shape == NodeShape::Table {
                    table_nodes.insert(container_id.clone());
                }
                if let Some(node) = graph.nodes.get_mut(container_id) {
                    node.shape = shape;
                } else {
                    let node = Node::with_shape(container_id.clone(), container_id.clone(), shape);
                    graph.nodes.insert(container_id.clone(), node);
                }
            }
            return;
        }
    }

    // Field declarations inside sql_table/class
    if let Some(container_id) = container_stack.last() {
        if table_nodes.contains(container_id) && !has_arrow(segment) && !segment.contains(".shape:")
        {
            let mut input = segment;
            if let Ok(field) = w_table_field(&mut input) {
                if let Some(node) = graph.nodes.get_mut(container_id) {
                    node.fields.push(field);
                }
                return;
            }
        }
    }

    // Constraint property inside container
    if !container_stack.is_empty() && segment.starts_with("constraint:") {
        return;
    }

    // Connections (may be chain)
    if has_arrow(segment) {
        parse_connection_chain(segment, graph, current_subgraph.as_deref(), container_stack);
        return;
    }

    // Shape property: id.shape: type
    {
        let mut input = segment;
        if let Ok((id, shape)) = w_shape_property(&mut input) {
            let resolved_id =
                resolve_dotted_id(&id, graph, container_stack, current_subgraph.as_deref());
            if shape == NodeShape::Table {
                table_nodes.insert(resolved_id.clone());
            }
            if let Some(node) = graph.nodes.get_mut(&resolved_id) {
                node.shape = shape;
            } else {
                let mut node = Node::with_shape(resolved_id.clone(), resolved_id.clone(), shape);
                node.subgraph = current_subgraph.clone();
                graph.nodes.insert(resolved_id, node);
            }
            return;
        }
    }

    // Label property: id.label: "text"
    {
        let mut input = segment;
        if let Ok((id, label)) = w_label_property(&mut input) {
            let resolved_id =
                resolve_dotted_id(&id, graph, container_stack, current_subgraph.as_deref());
            if let Some(node) = graph.nodes.get_mut(&resolved_id) {
                node.label = label;
            } else {
                let mut node = Node::new(resolved_id.clone(), label);
                node.subgraph = current_subgraph.clone();
                graph.nodes.insert(resolved_id, node);
            }
            return;
        }
    }

    // Skip other dotted properties
    if segment.contains('.') && segment.contains(':') {
        let dot_part = segment.split(':').next().unwrap_or("");
        if dot_part.contains('.') {
            let parts: Vec<&str> = dot_part.rsplitn(2, '.').collect();
            if parts.len() == 2 {
                let prop = parts[0].trim();
                match prop {
                    "shape" | "label" => {}
                    "style" | "near" | "tooltip" | "link" | "icon" => return,
                    _ if prop.starts_with("style") => return,
                    _ => {}
                }
            }
        }
    }

    // Node declaration or null deletion
    let (id, label) = parse_d2_label(segment);
    if id.is_empty() {
        return;
    }

    // Null deletion
    if label == "null" {
        let raw_after_id = segment[segment.find(&id).unwrap_or(0) + id.len()..].trim();
        if let Some(stripped) = raw_after_id.strip_prefix(':') {
            let val = stripped.trim();
            if val == "null" {
                null_nodes.push(id);
                return;
            }
        }
    }

    // Dotted id as nested node
    if id.contains('.') {
        let resolved = resolve_dotted_id(&id, graph, container_stack, current_subgraph.as_deref());
        use std::collections::hash_map::Entry;
        match graph.nodes.entry(resolved) {
            Entry::Occupied(mut e) => {
                e.get_mut().label = label;
            }
            Entry::Vacant(e) => {
                let leaf_subgraph = innermost_container_for_dotted(&id);
                let mut node = Node::new(e.key().clone(), label);
                node.subgraph = leaf_subgraph;
                e.insert(node);
            }
        }
        return;
    }

    let clean_id = strip_quotes(&id);

    use std::collections::hash_map::Entry;
    match graph.nodes.entry(clean_id.clone()) {
        Entry::Occupied(mut e) => {
            let clean_label = strip_quotes(&label);
            e.get_mut().label = clean_label;
        }
        Entry::Vacant(e) => {
            let clean_label = if label == id {
                clean_id.clone()
            } else {
                strip_quotes(&label)
            };
            let mut node = Node::new(e.key().clone(), clean_label);
            node.subgraph = current_subgraph;
            e.insert(node);
        }
    }
}

fn handle_container_open(
    container_def: &str,
    graph: &mut Graph,
    container_stack: &mut Vec<String>,
    _table_nodes: &mut std::collections::HashSet<String>,
) {
    let (raw_id, label) = parse_d2_label(container_def);
    let clean_id = strip_quotes(&raw_id);

    if clean_id.contains('.') {
        let parts: Vec<&str> = clean_id.split('.').collect();
        let mut parent: Option<String> = container_stack.last().cloned();

        for (i, part) in parts.iter().enumerate() {
            let part_id = part.to_string();
            let is_last = i == parts.len() - 1;
            let sg_label = if is_last && label != clean_id {
                label.clone()
            } else {
                part_id.clone()
            };

            if !graph.subgraphs.iter().any(|sg| sg.id == part_id) {
                let mut sg = Subgraph::new(part_id.clone(), sg_label);
                sg.parent = parent.clone();
                graph.subgraphs.push(sg);
            }

            parent = Some(part_id.clone());
            container_stack.push(part_id);
        }
    } else {
        let parent = container_stack.last().cloned();

        if !graph.subgraphs.iter().any(|sg| sg.id == clean_id) {
            let clean_label = strip_quotes(&label);
            let mut sg = Subgraph::new(clean_id.clone(), clean_label);
            sg.parent = parent;
            graph.subgraphs.push(sg);
        }

        container_stack.push(clean_id.clone());

        graph.nodes.entry(clean_id).or_insert_with_key(|id| {
            let clean_label = strip_quotes(&label);
            Node::new(id.clone(), clean_label)
        });
    }
}

fn check_unsupported(segment: &str, line_num: usize, warnings: &mut Vec<DiagramWarning>) -> bool {
    let lower = segment.to_lowercase();

    if lower.starts_with("...@") || lower.starts_with("import ") {
        warnings.push(DiagramWarning::UnsupportedFeature {
            feature: "import".to_string(),
            line: line_num,
        });
        return true;
    }

    if segment.contains('*')
        && !segment.contains('"')
        && !segment.contains('\'')
        && (segment.ends_with('*')
            || segment.contains(".*")
            || segment.contains("*.")
            || segment.trim() == "*")
    {
        warnings.push(DiagramWarning::UnsupportedFeature {
            feature: "glob".to_string(),
            line: line_num,
        });
        return true;
    }

    for keyword in &["layers", "scenarios", "steps"] {
        if lower.starts_with(&format!("{}:", keyword))
            || lower.starts_with(&format!("{} {{", keyword))
        {
            warnings.push(DiagramWarning::UnsupportedFeature {
                feature: keyword.to_string(),
                line: line_num,
            });
            return true;
        }
    }

    if lower.starts_with("grid-rows:") || lower.starts_with("grid-columns:") {
        warnings.push(DiagramWarning::UnsupportedFeature {
            feature: "grid layout".to_string(),
            line: line_num,
        });
        return true;
    }

    for keyword in &["tooltip:", "link:", "icon:"] {
        if lower.starts_with(keyword) {
            warnings.push(DiagramWarning::UnsupportedFeature {
                feature: keyword.trim_end_matches(':').to_string(),
                line: line_num,
            });
            return true;
        }
    }

    false
}

fn is_style_property(segment: &str) -> bool {
    let lower = segment.to_lowercase();
    (lower.contains("style.") && segment.contains(':')) || lower.starts_with("style:")
}

fn has_arrow(segment: &str) -> bool {
    let unquoted = strip_quoted_sections(segment);
    unquoted.contains("->")
        || unquoted.contains("<-")
        || (unquoted.contains("--") && !unquoted.contains("-->"))
}

fn strip_quoted_sections(s: &str) -> String {
    let mut result = String::new();
    let mut in_quote = false;
    let mut quote_char = '"';
    for c in s.chars() {
        if !in_quote && (c == '"' || c == '\'') {
            in_quote = true;
            quote_char = c;
            result.push(' ');
        } else if in_quote && c == quote_char {
            in_quote = false;
            result.push(' ');
        } else if in_quote {
            result.push(' ');
        } else {
            result.push(c);
        }
    }
    result
}

fn parse_connection_chain(
    segment: &str,
    graph: &mut Graph,
    current_subgraph: Option<&str>,
    container_stack: &[String],
) {
    let tokens = tokenize_connection(segment);
    if tokens.len() < 3 {
        if let Some((from, to, style, label)) = parse_d2_connection(segment) {
            let from_clean = resolve_connection_id(&from, graph, container_stack, current_subgraph);
            let to_clean = resolve_connection_id(&to, graph, container_stack, current_subgraph);
            ensure_node_exists(graph, &from_clean, current_subgraph);
            ensure_node_exists(graph, &to_clean, current_subgraph);
            graph.edges.push(Edge {
                from: from_clean,
                to: to_clean,
                label,
                style,
            });
        }
        return;
    }

    let mut i = 0;
    while i + 2 < tokens.len() {
        let from_raw = tokens[i].text.trim();
        let arrow = &tokens[i + 1];
        let to_raw = tokens[i + 2].text.trim();

        let style = arrow.style;
        let is_backward = arrow.text == "<-";

        let (to_id_raw, label) = if i + 2 == tokens.len() - 1 {
            parse_node_with_edge_label(to_raw)
        } else {
            (to_raw.to_string(), None)
        };

        let from_id = resolve_connection_id(
            &strip_quotes(from_raw),
            graph,
            container_stack,
            current_subgraph,
        );
        let to_id = resolve_connection_id(
            &strip_quotes(&to_id_raw),
            graph,
            container_stack,
            current_subgraph,
        );

        ensure_node_exists(graph, &from_id, current_subgraph);
        ensure_node_exists(graph, &to_id, current_subgraph);

        if is_backward {
            graph.edges.push(Edge {
                from: to_id,
                to: from_id,
                label,
                style,
            });
        } else {
            graph.edges.push(Edge {
                from: from_id,
                to: to_id,
                label,
                style,
            });
        }

        i += 2;
    }
}

struct ConnToken {
    text: String,
    style: EdgeStyle,
}

fn tokenize_connection(segment: &str) -> Vec<ConnToken> {
    let mut tokens: Vec<ConnToken> = Vec::new();
    let mut remaining = segment;

    loop {
        remaining = remaining.trim();
        if remaining.is_empty() {
            break;
        }

        if let Some((before, arrow, style, after)) = find_next_arrow(remaining) {
            let node_text = before.trim();
            if !node_text.is_empty() {
                tokens.push(ConnToken {
                    text: node_text.to_string(),
                    style: EdgeStyle::Arrow,
                });
            }
            tokens.push(ConnToken {
                text: arrow.to_string(),
                style,
            });
            remaining = after;
        } else {
            let node_text = remaining.trim();
            if !node_text.is_empty() {
                tokens.push(ConnToken {
                    text: node_text.to_string(),
                    style: EdgeStyle::Arrow,
                });
            }
            break;
        }
    }

    tokens
}

fn find_next_arrow(s: &str) -> Option<(&str, &str, EdgeStyle, &str)> {
    let mut in_quote = false;
    let mut quote_char = '"';

    // Use char_indices to safely handle multi-byte UTF-8
    let chars: Vec<(usize, char)> = s.char_indices().collect();

    for (ci, &(byte_pos, c)) in chars.iter().enumerate() {
        if !in_quote && (c == '"' || c == '\'') {
            in_quote = true;
            quote_char = c;
            continue;
        }
        if in_quote && c == quote_char {
            in_quote = false;
            continue;
        }
        if in_quote {
            continue;
        }

        // Check for arrow patterns using char lookahead
        if c == '<' && ci + 2 < chars.len() && chars[ci + 1].1 == '-' && chars[ci + 2].1 == '>' {
            let end_byte = chars[ci + 2].0 + chars[ci + 2].1.len_utf8();
            return Some((&s[..byte_pos], "<->", EdgeStyle::Arrow, &s[end_byte..]));
        }
        if c == '-' && ci + 1 < chars.len() && chars[ci + 1].1 == '>' {
            let end_byte = chars[ci + 1].0 + chars[ci + 1].1.len_utf8();
            return Some((&s[..byte_pos], "->", EdgeStyle::Arrow, &s[end_byte..]));
        }
        if c == '<' && ci + 1 < chars.len() && chars[ci + 1].1 == '-' {
            // Make sure it's not <->
            if ci + 2 < chars.len() && chars[ci + 2].1 == '>' {
                continue; // handled above
            }
            let end_byte = chars[ci + 1].0 + chars[ci + 1].1.len_utf8();
            return Some((&s[..byte_pos], "<-", EdgeStyle::Arrow, &s[end_byte..]));
        }
        if c == '-' && ci + 1 < chars.len() && chars[ci + 1].1 == '-' {
            // Make sure it's not --> (mermaid)
            if ci + 2 < chars.len() && chars[ci + 2].1 == '>' {
                continue;
            }
            let end_byte = chars[ci + 1].0 + chars[ci + 1].1.len_utf8();
            return Some((&s[..byte_pos], "--", EdgeStyle::Line, &s[end_byte..]));
        }
    }

    None
}

fn parse_node_with_edge_label(s: &str) -> (String, Option<String>) {
    let mut in_quote = false;
    let mut quote_char = '"';

    for (i, c) in s.char_indices() {
        if !in_quote && (c == '"' || c == '\'') {
            in_quote = true;
            quote_char = c;
            continue;
        }
        if in_quote && c == quote_char {
            in_quote = false;
            continue;
        }
        if !in_quote && c == ':' {
            let node_id = s[..i].trim().to_string();
            let label = s[i + 1..]
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .to_string();
            if label.is_empty() {
                return (node_id, None);
            }
            return (node_id, Some(label));
        }
    }

    (s.to_string(), None)
}

fn resolve_connection_id(
    id: &str,
    graph: &mut Graph,
    container_stack: &[String],
    current_subgraph: Option<&str>,
) -> String {
    let clean = strip_quotes(id);
    if clean.contains('.') {
        resolve_dotted_id(&clean, graph, container_stack, current_subgraph)
    } else {
        clean
    }
}

fn resolve_dotted_id(
    dotted: &str,
    graph: &mut Graph,
    _container_stack: &[String],
    current_subgraph: Option<&str>,
) -> String {
    let parts: Vec<&str> = dotted.split('.').collect();
    if parts.len() <= 1 {
        return strip_quotes(dotted);
    }

    let mut parent: Option<String> = current_subgraph.map(String::from);

    for part in parts.iter().take(parts.len() - 1) {
        let part_id = strip_quotes(part);

        if !graph.subgraphs.iter().any(|sg| sg.id == part_id) {
            let mut sg = Subgraph::new(part_id.clone(), part_id.clone());
            sg.parent = parent.clone();
            graph.subgraphs.push(sg);
        }

        if !graph.nodes.contains_key(&part_id) {
            let mut node = Node::new(part_id.clone(), part_id.clone());
            node.subgraph = parent.clone();
            graph.nodes.insert(part_id.clone(), node);
        }

        parent = Some(part_id);
    }

    // Safety: parts.len() > 1 guaranteed by early return above
    let leaf_id = strip_quotes(parts.last().expect("parts has >= 2 elements"));

    if !graph.nodes.contains_key(&leaf_id) {
        let mut node = Node::new(leaf_id.clone(), leaf_id.clone());
        node.subgraph = parent.clone();
        graph.nodes.insert(leaf_id.clone(), node);
    }

    if let Some(parent_id) = parts.get(parts.len() - 2).map(|s| strip_quotes(s)) {
        if let Some(sg) = graph.subgraphs.iter_mut().find(|sg| sg.id == parent_id) {
            if !sg.nodes.contains(&leaf_id) {
                sg.nodes.push(leaf_id.clone());
            }
        }
    }

    leaf_id
}

fn innermost_container_for_dotted(dotted: &str) -> Option<String> {
    let parts: Vec<&str> = dotted.split('.').collect();
    if parts.len() <= 1 {
        return None;
    }
    Some(strip_quotes(parts[parts.len() - 2]))
}

fn split_on_semicolons(line: &str) -> Vec<&str> {
    let mut segments = Vec::new();
    let mut start = 0;
    let mut in_quote = false;
    let mut quote_char = '"';
    let mut brace_depth = 0;

    for (i, c) in line.char_indices() {
        if !in_quote && (c == '"' || c == '\'') {
            in_quote = true;
            quote_char = c;
        } else if in_quote && c == quote_char {
            in_quote = false;
        } else if !in_quote && c == '{' {
            brace_depth += 1;
        } else if !in_quote && c == '}' {
            brace_depth -= 1;
        } else if !in_quote && brace_depth == 0 && c == ';' {
            segments.push(&line[start..i]);
            start = i + 1;
        }
    }

    if start < line.len() {
        segments.push(&line[start..]);
    }

    segments
}

fn ensure_node_exists(graph: &mut Graph, id: &str, subgraph: Option<&str>) {
    if graph.nodes.contains_key(id) {
        return;
    }
    let mut node = Node::new(id.to_string(), id.to_string());
    node.subgraph = subgraph.map(String::from);
    graph.nodes.insert(id.to_string(), node);

    if let Some(sg_id) = subgraph {
        if let Some(sg) = graph.subgraphs.iter_mut().find(|sg| sg.id == sg_id) {
            if !sg.nodes.contains(&id.to_string()) {
                sg.nodes.push(id.to_string());
            }
        }
    }
}

fn parse_d2_label(s: &str) -> (String, String) {
    let s = if let Some(idx) = s.find(';') {
        s[..idx].trim()
    } else {
        s
    };

    let mut in_quote = false;
    let mut quote_char = '"';

    for (i, c) in s.char_indices() {
        if !in_quote && (c == '"' || c == '\'') {
            in_quote = true;
            quote_char = c;
            continue;
        }
        if in_quote && c == quote_char {
            in_quote = false;
            continue;
        }
        if !in_quote && c == ':' {
            let id = s[..i].trim().to_string();
            let label = s[i + 1..]
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .to_string();
            let clean_id = strip_quotes(&id);
            let final_label = if label.is_empty() {
                clean_id.clone()
            } else {
                label
            };
            return (clean_id, final_label);
        }
    }

    let id = s.trim().to_string();
    let clean_id = strip_quotes(&id);
    (clean_id.clone(), clean_id)
}

fn parse_d2_connection(line: &str) -> Option<(NodeId, NodeId, EdgeStyle, Option<String>)> {
    let patterns = [
        ("<->", EdgeStyle::Arrow, true),
        ("->", EdgeStyle::Arrow, false),
        ("<-", EdgeStyle::Arrow, false),
        ("--", EdgeStyle::Line, false),
    ];

    for (pattern, style, _is_bidirectional) in patterns {
        if let Some(idx) = find_arrow_in_line(line, pattern) {
            let left = line[..idx].trim();
            let right_part = line[idx + pattern.len()..].trim();

            let (to, label) = parse_node_with_edge_label(right_part);

            let from = left.to_string();

            if pattern == "<-" {
                return Some((to, from, style, label));
            }

            return Some((from, to, style, label));
        }
    }

    None
}

fn find_arrow_in_line(line: &str, pattern: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    let pat_bytes = pattern.as_bytes();
    let pat_len = pat_bytes.len();
    let len = bytes.len();

    if len < pat_len {
        return None;
    }

    let mut in_quote = false;
    let mut quote_char = b'"';

    for i in 0..=len - pat_len {
        let c = bytes[i];
        if !in_quote && (c == b'"' || c == b'\'') {
            in_quote = true;
            quote_char = c;
            continue;
        }
        if in_quote && c == quote_char {
            in_quote = false;
            continue;
        }
        if in_quote {
            continue;
        }

        if &bytes[i..i + pat_len] == pat_bytes {
            if pattern == "--" && i + pat_len < len && bytes[i + pat_len] == b'>' {
                continue;
            }
            return Some(i);
        }
    }

    None
}

fn parse_shape_str(shape_str: &str) -> NodeShape {
    match shape_str {
        "rectangle" | "rect" => NodeShape::Rectangle,
        "square" => NodeShape::Rectangle,
        "circle" => NodeShape::Circle,
        "oval" | "ellipse" => NodeShape::Rounded,
        "diamond" => NodeShape::Diamond,
        "cylinder" | "queue" | "stored_data" => NodeShape::Cylinder,
        "hexagon" => NodeShape::Hexagon,
        "parallelogram" | "step" => NodeShape::Parallelogram,
        "document" | "page" => NodeShape::Document,
        "package" => NodeShape::Rectangle,
        "cloud" => NodeShape::Cloud,
        "person" => NodeShape::Person,
        "sql_table" | "class" => NodeShape::Table,
        _ => NodeShape::Rectangle,
    }
}

fn strip_quotes(s: &str) -> String {
    let s = s.trim();
    if s.len() >= 2
        && ((s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')))
    {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> (Graph, Vec<DiagramWarning>) {
        let result = parse_d2(input).unwrap();
        (result.graph, result.warnings)
    }

    #[test]
    fn test_parse_d2_simple() {
        let (graph, _) = parse("A -> B");
        assert!(graph.nodes.contains_key("A"));
        assert!(graph.nodes.contains_key("B"));
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.edges[0].from, "A");
        assert_eq!(graph.edges[0].to, "B");
    }

    #[test]
    fn test_parse_d2_with_labels() {
        let (graph, _) = parse(
            r#"
server: "Web Server"
db: Database
server -> db
"#,
        );
        assert_eq!(graph.nodes.get("server").unwrap().label, "Web Server");
        assert_eq!(graph.nodes.get("db").unwrap().label, "Database");
    }

    #[test]
    fn test_parse_d2_edge_label() {
        let (graph, _) = parse("A -> B: \"HTTP request\"");
        assert_eq!(graph.edges[0].label, Some("HTTP request".to_string()));
    }

    #[test]
    fn test_parse_d2_chain_separate_lines() {
        let (graph, _) = parse(
            r#"
A -> B
B -> C
C -> D
"#,
        );
        assert_eq!(graph.edges.len(), 3);
    }

    #[test]
    fn test_parse_d2_connection_chain() {
        let (graph, _) = parse("A -> B -> C -> D");
        assert_eq!(graph.edges.len(), 3);
        assert_eq!(graph.edges[0].from, "A");
        assert_eq!(graph.edges[0].to, "B");
        assert_eq!(graph.edges[1].from, "B");
        assert_eq!(graph.edges[1].to, "C");
        assert_eq!(graph.edges[2].from, "C");
        assert_eq!(graph.edges[2].to, "D");
        assert_eq!(graph.nodes.len(), 4);
    }

    #[test]
    fn test_parse_d2_backward_arrow() {
        let (graph, _) = parse("A <- B");
        assert_eq!(graph.edges[0].from, "B");
        assert_eq!(graph.edges[0].to, "A");
    }

    #[test]
    fn test_parse_d2_line() {
        let (graph, _) = parse("A -- B");
        assert!(matches!(graph.edges[0].style, EdgeStyle::Line));
    }

    #[test]
    fn test_parse_d2_shape_property() {
        let (graph, _) = parse(
            r#"
db: Database
db.shape: cylinder
"#,
        );
        assert!(matches!(
            graph.nodes.get("db").unwrap().shape,
            NodeShape::Cylinder
        ));
    }

    #[test]
    fn test_parse_d2_sql_table() {
        let (graph, _) = parse(
            r#"
users: Users Table
users.shape: sql_table
"#,
        );
        assert!(matches!(
            graph.nodes.get("users").unwrap().shape,
            NodeShape::Table
        ));
    }

    #[test]
    fn test_parse_d2_container() {
        let (graph, _) = parse(
            r#"
backend {
    api: "API Server"
    db: Database
}
api -> db
"#,
        );
        assert!(graph.subgraphs.iter().any(|sg| sg.id == "backend"));
        assert_eq!(
            graph.nodes.get("api").unwrap().subgraph,
            Some("backend".to_string())
        );
    }

    #[test]
    fn test_parse_d2_comments() {
        let (graph, _) = parse(
            r#"
# This is a comment
A -> B
"#,
        );
        assert_eq!(graph.edges.len(), 1);
    }

    #[test]
    fn test_parse_d2_empty() {
        let result = parse_d2("");
        assert!(matches!(result, Err(MermaidError::EmptyInput)));
    }

    #[test]
    fn test_parse_d2_style_and_direction_not_nodes() {
        let (graph, _) = parse(
            r##"
direction: right

input: Raw Data Block {
  shape: document
}

center: Statistical Center {
  shape: diamond
  style.fill: "#4CAF50"
}

forward: Forward Stream {
  shape: hexagon
  style.fill: "#2196F3"
}

input -> center: Find center
center -> forward: center → end
"##,
        );

        assert!(matches!(graph.direction, Direction::LR));
        assert_eq!(graph.nodes.len(), 3);
        assert!(graph.nodes.contains_key("input"));
        assert!(graph.nodes.contains_key("center"));
        assert!(graph.nodes.contains_key("forward"));

        assert!(!graph.nodes.contains_key("right"));
        assert!(!graph.nodes.contains_key("document"));
        assert!(!graph.nodes.contains_key("diamond"));
        assert!(!graph.nodes.contains_key("hexagon"));

        assert!(matches!(
            graph.nodes.get("center").unwrap().shape,
            NodeShape::Diamond
        ));
        assert!(matches!(
            graph.nodes.get("forward").unwrap().shape,
            NodeShape::Hexagon
        ));
    }

    #[test]
    fn test_parse_d2_semicolons() {
        let (graph, _) = parse("A -> B; C -> D");
        assert_eq!(graph.edges.len(), 2);
        assert_eq!(graph.edges[0].from, "A");
        assert_eq!(graph.edges[0].to, "B");
        assert_eq!(graph.edges[1].from, "C");
        assert_eq!(graph.edges[1].to, "D");
    }

    #[test]
    fn test_parse_d2_nested_containers() {
        let (graph, _) = parse(
            r#"
cloud {
    backend {
        api: API
        db: Database
    }
    frontend {
        web: Web App
    }
}
api -> db
web -> api
"#,
        );
        assert!(graph.subgraphs.iter().any(|sg| sg.id == "cloud"));
        assert!(graph.subgraphs.iter().any(|sg| sg.id == "backend"));
        assert!(graph.subgraphs.iter().any(|sg| sg.id == "frontend"));
        let backend_sg = graph
            .subgraphs
            .iter()
            .find(|sg| sg.id == "backend")
            .unwrap();
        assert_eq!(backend_sg.parent, Some("cloud".to_string()));
    }

    #[test]
    fn test_parse_d2_dotted_key_paths() {
        let (graph, _) = parse("a.b.c -> d.e.f");
        assert!(graph.nodes.contains_key("c"));
        assert!(graph.nodes.contains_key("f"));
        assert!(graph.subgraphs.iter().any(|sg| sg.id == "a"));
        assert!(graph.subgraphs.iter().any(|sg| sg.id == "b"));
        assert!(graph.subgraphs.iter().any(|sg| sg.id == "d"));
        assert!(graph.subgraphs.iter().any(|sg| sg.id == "e"));
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.edges[0].from, "c");
        assert_eq!(graph.edges[0].to, "f");
    }

    #[test]
    fn test_parse_d2_quoted_keys() {
        let (graph, _) = parse(r#""my node" -> "other node""#);
        assert!(graph.nodes.contains_key("my node"));
        assert!(graph.nodes.contains_key("other node"));
        assert_eq!(graph.edges[0].from, "my node");
        assert_eq!(graph.edges[0].to, "other node");
    }

    #[test]
    fn test_parse_d2_null_deletion() {
        let (graph, _) = parse(
            r#"
A -> B
B -> C
B: null
"#,
        );
        assert!(!graph.nodes.contains_key("B"));
        assert!(graph.edges.is_empty());
    }

    #[test]
    fn test_parse_d2_sql_table_fields() {
        let (graph, _) = parse(
            r#"
users {
    shape: sql_table
    id: int {constraint: primary_key}
    name: varchar
    email: varchar
}
"#,
        );
        let users = graph.nodes.get("users").unwrap();
        assert!(matches!(users.shape, NodeShape::Table));
        assert_eq!(users.fields.len(), 3);
        assert_eq!(users.fields[0].name, "id");
        assert_eq!(users.fields[0].type_info, Some("int".to_string()));
        assert_eq!(users.fields[0].constraint, Some("primary_key".to_string()));
        assert_eq!(users.fields[1].name, "name");
        assert_eq!(users.fields[2].name, "email");
    }

    #[test]
    fn test_parse_d2_unsupported_glob() {
        let (_, warnings) = parse(
            r#"
A -> B
*.style.fill: red
"#,
        );
        assert!(warnings.iter().any(|w| matches!(
            w,
            DiagramWarning::UnsupportedFeature { feature, .. } if feature == "glob"
        )));
    }

    #[test]
    fn test_parse_d2_unsupported_layers() {
        let (_, warnings) = parse(
            r#"
A -> B
layers: {
}
"#,
        );
        assert!(warnings.iter().any(|w| matches!(
            w,
            DiagramWarning::UnsupportedFeature { feature, .. } if feature == "layers"
        )));
    }

    #[test]
    fn test_parse_d2_unsupported_tooltip() {
        let (_, warnings) = parse(
            r#"
A -> B
tooltip: "some tooltip"
"#,
        );
        assert!(warnings.iter().any(|w| matches!(
            w,
            DiagramWarning::UnsupportedFeature { feature, .. } if feature == "tooltip"
        )));
    }

    #[test]
    fn test_parse_d2_label_update() {
        let (graph, _) = parse(
            r#"
A: First Label
A: Updated Label
"#,
        );
        assert_eq!(graph.nodes.get("A").unwrap().label, "Updated Label");
        assert_eq!(graph.nodes.len(), 1);
    }

    #[test]
    fn test_parse_d2_bidirectional() {
        let (graph, _) = parse("A <-> B");
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.edges[0].from, "A");
        assert_eq!(graph.edges[0].to, "B");
    }

    #[test]
    fn test_parse_d2_mixed_features() {
        let (graph, warnings) = parse(
            r#"
direction: right

# Network diagram
cloud: Cloud Provider {
    api: API Gateway {
        shape: hexagon
    }
    db: Database {
        shape: cylinder
    }
}

client: Client App
client -> api: REST
api -> db: SQL

tooltip: "hover text"
"#,
        );

        assert!(matches!(graph.direction, Direction::LR));
        assert!(graph.nodes.contains_key("client"));
        assert!(graph.nodes.contains_key("api"));
        assert!(graph.nodes.contains_key("db"));
        assert_eq!(graph.edges.len(), 2);

        assert!(warnings.iter().any(|w| matches!(
            w,
            DiagramWarning::UnsupportedFeature { feature, .. } if feature == "tooltip"
        )));
    }

    #[test]
    fn test_parse_d2_dotted_shape_property() {
        let (graph, _) = parse(
            r#"
server.shape: hexagon
server: My Server
"#,
        );
        assert!(matches!(
            graph.nodes.get("server").unwrap().shape,
            NodeShape::Hexagon
        ));
        assert_eq!(graph.nodes.get("server").unwrap().label, "My Server");
    }

    #[test]
    fn test_parse_d2_chain_with_label() {
        let (graph, _) = parse("A -> B -> C: final");
        assert_eq!(graph.edges.len(), 2);
        assert_eq!(graph.edges[0].from, "A");
        assert_eq!(graph.edges[0].to, "B");
        assert_eq!(graph.edges[1].from, "B");
        assert_eq!(graph.edges[1].to, "C");
        assert_eq!(graph.edges[1].label, Some("final".to_string()));
    }
}
