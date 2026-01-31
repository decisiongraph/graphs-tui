//! State diagram parser for Mermaid syntax
//!
//! Supports both stateDiagram (v1) and stateDiagram-v2 syntax

use crate::error::MermaidError;
use crate::types::{Direction, Edge, EdgeStyle, Graph, Node, NodeShape, Subgraph};

/// Parse state diagram syntax into a Graph
pub fn parse_state_diagram(input: &str) -> Result<Graph, MermaidError> {
    let lines: Vec<&str> = input
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with("%%"))
        .collect();

    if lines.is_empty() {
        return Err(MermaidError::EmptyInput);
    }

    // Validate header
    let first_line = lines[0].to_lowercase();
    if !first_line.starts_with("statediagram") {
        return Err(MermaidError::ParseError {
            line: 1,
            message: "Expected stateDiagram or stateDiagram-v2".to_string(),
            suggestion: Some("Start with 'stateDiagram' or 'stateDiagram-v2'".to_string()),
        });
    }

    // State diagrams are typically vertical (TB)
    let mut graph = Graph::new(Direction::TB);
    let mut current_composite: Option<String> = None;
    let mut state_counter = 0;

    for line in lines.iter().skip(1) {
        // Skip direction declarations
        if line.starts_with("direction") {
            continue;
        }

        // Handle composite state start: state Name {
        if line.starts_with("state") && line.ends_with('{') {
            let (id, label) = parse_state_declaration(line)?;
            let sg = Subgraph::new(id.clone(), label);
            graph.subgraphs.push(sg);
            current_composite = Some(id);
            continue;
        }

        // Handle composite state end
        if *line == "}" {
            current_composite = None;
            continue;
        }

        // Handle state declaration with description: state "Description" as ID
        if line.starts_with("state") {
            let (id, label) = parse_state_declaration(line)?;
            let mut node = Node::with_shape(id.clone(), label, NodeShape::Rounded);
            node.subgraph = current_composite.clone();
            graph.nodes.insert(id, node);
            continue;
        }

        // Handle transitions: State1 --> State2 or State1 --> State2: label
        if line.contains("-->") {
            parse_transition(
                &mut graph,
                line,
                current_composite.as_deref(),
                &mut state_counter,
            )?;
            continue;
        }

        // Handle simple state declaration (just an ID)
        if is_valid_state_id(line) {
            let id = line.to_string();
            graph.nodes.entry(id).or_insert_with_key(|key| {
                let mut node = Node::with_shape(key.clone(), key.clone(), NodeShape::Rounded);
                node.subgraph = current_composite.clone();
                node
            });
        }
    }

    if graph.nodes.is_empty() && graph.edges.is_empty() {
        return Err(MermaidError::ParseError {
            line: 1,
            message: "No valid state diagram content".to_string(),
            suggestion: Some("Add states and transitions like 'State1 --> State2'".to_string()),
        });
    }

    Ok(graph)
}

/// Parse state declaration: state "Description" as ID or state ID { or just state ID
fn parse_state_declaration(line: &str) -> Result<(String, String), MermaidError> {
    let rest = line.strip_prefix("state").unwrap_or(line).trim();

    // Handle: state Name {
    if rest.ends_with('{') {
        let name = rest.trim_end_matches('{').trim();
        return Ok((name.to_string(), name.to_string()));
    }

    // Handle: state "Description" as ID
    if let Some(stripped) = rest.strip_prefix('"') {
        if let Some(end_quote) = stripped.find('"') {
            let description = &stripped[..end_quote];
            let after_quote = stripped[end_quote + 1..].trim();
            if after_quote.starts_with("as") {
                let id = after_quote.strip_prefix("as").unwrap_or("").trim();
                return Ok((id.to_string(), description.to_string()));
            }
        }
    }

    // Handle: state ID
    let id = rest.split_whitespace().next().unwrap_or(rest);
    Ok((id.to_string(), id.to_string()))
}

/// Parse transition line: State1 --> State2 or State1 --> State2: label
fn parse_transition(
    graph: &mut Graph,
    line: &str,
    current_composite: Option<&str>,
    state_counter: &mut usize,
) -> Result<(), MermaidError> {
    let parts: Vec<&str> = line.splitn(2, "-->").collect();
    if parts.len() != 2 {
        return Ok(());
    }

    let from_raw = parts[0].trim();
    let to_part = parts[1].trim();

    // Check for label: State : label
    let (to_raw, label) = if let Some(colon_idx) = to_part.find(':') {
        let to = to_part[..colon_idx].trim();
        let lbl = to_part[colon_idx + 1..].trim();
        (to, Some(lbl.to_string()))
    } else {
        (to_part, None)
    };

    // Handle special states [*] for start/end
    let from = if from_raw == "[*]" {
        *state_counter += 1;
        let id = format!("__start_{}", state_counter);
        let mut node = Node::with_shape(id.clone(), "●".to_string(), NodeShape::Circle);
        node.subgraph = current_composite.map(String::from);
        graph.nodes.insert(id.clone(), node);
        id
    } else {
        ensure_state_exists(graph, from_raw, current_composite);
        from_raw.to_string()
    };

    let to = if to_raw == "[*]" {
        *state_counter += 1;
        let id = format!("__end_{}", state_counter);
        let mut node = Node::with_shape(id.clone(), "◉".to_string(), NodeShape::Circle);
        node.subgraph = current_composite.map(String::from);
        graph.nodes.insert(id.clone(), node);
        id
    } else {
        ensure_state_exists(graph, to_raw, current_composite);
        to_raw.to_string()
    };

    graph.edges.push(Edge {
        from,
        to,
        label,
        style: EdgeStyle::Arrow,
    });

    Ok(())
}

/// Ensure a state exists in the graph
fn ensure_state_exists(graph: &mut Graph, id: &str, composite: Option<&str>) {
    if !graph.nodes.contains_key(id) {
        let mut node = Node::with_shape(id.to_string(), id.to_string(), NodeShape::Rounded);
        node.subgraph = composite.map(String::from);
        graph.nodes.insert(id.to_string(), node);
    }
}

/// Check if string is a valid state ID
fn is_valid_state_id(s: &str) -> bool {
    !s.is_empty()
        && !s.contains("-->")
        && !s.contains(':')
        && !s.starts_with('[')
        && !s.starts_with('{')
        && !s.ends_with('}')
        && s.chars().all(|c| c.is_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_transition() {
        let input = "stateDiagram\n    s1 --> s2";
        let graph = parse_state_diagram(input).unwrap();
        assert!(graph.nodes.contains_key("s1"));
        assert!(graph.nodes.contains_key("s2"));
        assert_eq!(graph.edges.len(), 1);
    }

    #[test]
    fn test_parse_start_end_states() {
        let input = "stateDiagram-v2\n    [*] --> Idle\n    Idle --> [*]";
        let graph = parse_state_diagram(input).unwrap();
        assert!(graph.nodes.contains_key("Idle"));
        assert_eq!(graph.edges.len(), 2);
    }

    #[test]
    fn test_parse_state_description() {
        let input = "stateDiagram-v2\n    state \"Waiting\" as Wait\n    Wait --> Done";
        let graph = parse_state_diagram(input).unwrap();
        assert_eq!(graph.nodes.get("Wait").unwrap().label, "Waiting");
    }

    #[test]
    fn test_parse_transition_label() {
        let input = "stateDiagram-v2\n    Idle --> Running: start";
        let graph = parse_state_diagram(input).unwrap();
        assert_eq!(graph.edges[0].label, Some("start".to_string()));
    }

    #[test]
    fn test_parse_composite_state() {
        let input = "stateDiagram-v2\n    state Active {\n        Running --> Paused\n    }";
        let graph = parse_state_diagram(input).unwrap();
        assert_eq!(graph.subgraphs.len(), 1);
        assert_eq!(graph.subgraphs[0].id, "Active");
    }
}
