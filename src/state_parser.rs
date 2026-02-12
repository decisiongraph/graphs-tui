//! State diagram parser for Mermaid syntax
//!
//! Supports both stateDiagram (v1) and stateDiagram-v2 syntax

use winnow::ascii::{space0, space1};
use winnow::combinator::{alt, delimited, opt, preceded};
use winnow::token::{rest, take_until, take_while};
use winnow::PResult;
use winnow::Parser;

use crate::error::MermaidError;
use crate::types::{Direction, Edge, EdgeStyle, Graph, Node, NodeShape, Subgraph};

/// Content of a single line (after trimming)
#[derive(Debug)]
enum StateLine {
    Header,
    Direction,
    StateDeclaration {
        id: String,
        label: String,
    },
    CompositeStart {
        id: String,
        label: String,
    },
    CompositeEnd,
    Transition {
        from: String,
        to: String,
        label: Option<String>,
    },
    SimpleState(String),
    Empty,
}

/// Parse stateDiagram or stateDiagram-v2 header
fn parse_header(input: &mut &str) -> PResult<()> {
    let _ = winnow::ascii::Caseless("statediagram").parse_next(input)?;
    let _ = opt(("-v2", opt(space0))).parse_next(input)?;
    Ok(())
}

/// Parse direction declaration
fn parse_direction(input: &mut &str) -> PResult<()> {
    let _ = winnow::ascii::Caseless("direction").parse_next(input)?;
    Ok(())
}

/// Parse a quoted string: "..."
fn parse_quoted_string(input: &mut &str) -> PResult<String> {
    delimited('"', take_until(0.., "\""), '"')
        .map(|s: &str| s.to_string())
        .parse_next(input)
}

/// Parse state ID (alphanumeric + underscore)
fn parse_state_id(input: &mut &str) -> PResult<String> {
    take_while(1.., |c: char| c.is_alphanumeric() || c == '_')
        .map(|s: &str| s.to_string())
        .parse_next(input)
}

/// Parse [*] special state marker
fn parse_special_state(input: &mut &str) -> PResult<String> {
    delimited('[', '*', ']')
        .map(|_| "[*]".to_string())
        .parse_next(input)
}

/// Parse a state reference (either [*] or regular ID)
fn parse_state_ref(input: &mut &str) -> PResult<String> {
    alt((parse_special_state, parse_state_id)).parse_next(input)
}

/// Parse state declaration: state "Description" as ID
fn parse_state_with_description(input: &mut &str) -> PResult<(String, String)> {
    let _ = winnow::ascii::Caseless("state").parse_next(input)?;
    let _ = space1.parse_next(input)?;
    let description = parse_quoted_string.parse_next(input)?;
    let _ = space1.parse_next(input)?;
    let _ = winnow::ascii::Caseless("as").parse_next(input)?;
    let _ = space1.parse_next(input)?;
    let id = parse_state_id.parse_next(input)?;
    Ok((id, description))
}

/// Parse composite state start: state Name {
fn parse_composite_start(input: &mut &str) -> PResult<String> {
    let _ = winnow::ascii::Caseless("state").parse_next(input)?;
    let _ = space1.parse_next(input)?;
    let name = take_while(1.., |c: char| c.is_alphanumeric() || c == '_').parse_next(input)?;
    let _ = space0.parse_next(input)?;
    let _ = '{'.parse_next(input)?;
    Ok(name.to_string())
}

/// Parse simple state declaration: state ID
fn parse_simple_state_decl(input: &mut &str) -> PResult<String> {
    let _ = winnow::ascii::Caseless("state").parse_next(input)?;
    let _ = space1.parse_next(input)?;
    let id = parse_state_id.parse_next(input)?;
    Ok(id)
}

/// Parse transition: State1 --> State2 or State1 --> State2: label
fn parse_transition(input: &mut &str) -> PResult<(String, String, Option<String>)> {
    let from = parse_state_ref.parse_next(input)?;
    let _ = space0.parse_next(input)?;
    let _ = "-->".parse_next(input)?;
    let _ = space0.parse_next(input)?;
    let to = parse_state_ref.parse_next(input)?;

    // Check for label
    let _ = space0.parse_next(input)?;
    let label = opt(preceded(':', preceded(space0, rest)))
        .map(|o: Option<&str>| o.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()))
        .parse_next(input)?;

    Ok((from, to, label))
}

/// Parse a single line and classify it
fn parse_line(line: &str) -> StateLine {
    let trimmed = line.trim();

    // Empty line
    if trimmed.is_empty() {
        return StateLine::Empty;
    }

    // Comment
    if trimmed.starts_with("%%") {
        return StateLine::Empty;
    }

    // Composite state end
    if trimmed == "}" {
        return StateLine::CompositeEnd;
    }

    // Header
    if parse_header.parse(trimmed).is_ok() {
        return StateLine::Header;
    }

    // Direction
    if parse_direction.parse(trimmed).is_ok() {
        return StateLine::Direction;
    }

    // Composite state start
    if let Ok(id) = parse_composite_start.parse(trimmed) {
        return StateLine::CompositeStart {
            id: id.clone(),
            label: id,
        };
    }

    // State with description
    if let Ok((id, label)) = parse_state_with_description.parse(trimmed) {
        return StateLine::StateDeclaration { id, label };
    }

    // Transition
    if let Ok((from, to, label)) = parse_transition.parse(trimmed) {
        return StateLine::Transition { from, to, label };
    }

    // Simple state declaration
    if let Ok(id) = parse_simple_state_decl.parse(trimmed) {
        return StateLine::StateDeclaration {
            id: id.clone(),
            label: id,
        };
    }

    // Check if it's a simple valid state ID
    if is_valid_state_id(trimmed) {
        return StateLine::SimpleState(trimmed.to_string());
    }

    StateLine::Empty
}

/// Parse state diagram syntax into a Graph
pub fn parse_state_diagram(input: &str) -> Result<Graph, MermaidError> {
    let lines: Vec<&str> = input.lines().collect();

    if lines.is_empty() || lines.iter().all(|l| l.trim().is_empty()) {
        return Err(MermaidError::EmptyInput);
    }

    let mut graph = Graph::new(Direction::TB);
    let mut current_composite: Option<String> = None;
    let mut state_counter = 0;
    let mut found_header = false;

    for line in lines.iter() {
        match parse_line(line) {
            StateLine::Header => {
                found_header = true;
            }
            StateLine::Direction => {}
            StateLine::StateDeclaration { id, label } => {
                let mut node = Node::with_shape(id.clone(), label, NodeShape::Rounded);
                node.subgraph = current_composite.clone();
                graph.nodes.insert(id, node);
            }
            StateLine::CompositeStart { id, label } => {
                let sg = Subgraph::new(id.clone(), label);
                graph.subgraphs.push(sg);
                current_composite = Some(id);
            }
            StateLine::CompositeEnd => {
                current_composite = None;
            }
            StateLine::Transition { from, to, label } => {
                let from_id = handle_state_ref(
                    &mut graph,
                    &from,
                    current_composite.as_deref(),
                    &mut state_counter,
                    true,
                );
                let to_id = handle_state_ref(
                    &mut graph,
                    &to,
                    current_composite.as_deref(),
                    &mut state_counter,
                    false,
                );
                graph.edges.push(Edge {
                    from: from_id,
                    to: to_id,
                    label,
                    style: EdgeStyle::Arrow,
                });
            }
            StateLine::SimpleState(id) => {
                graph.nodes.entry(id.clone()).or_insert_with(|| {
                    let mut node = Node::with_shape(id.clone(), id.clone(), NodeShape::Rounded);
                    node.subgraph = current_composite.clone();
                    node
                });
            }
            StateLine::Empty => {}
        }
    }

    if !found_header {
        return Err(MermaidError::ParseError {
            line: 1,
            message: "Expected stateDiagram or stateDiagram-v2".to_string(),
            suggestion: Some("Start with 'stateDiagram' or 'stateDiagram-v2'".to_string()),
        });
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

/// Handle a state reference, creating special nodes for [*]
fn handle_state_ref(
    graph: &mut Graph,
    state_ref: &str,
    composite: Option<&str>,
    counter: &mut usize,
    is_start: bool,
) -> String {
    if state_ref == "[*]" {
        *counter += 1;
        let (id, label) = if is_start {
            (format!("__start_{}", counter), "●".to_string())
        } else {
            (format!("__end_{}", counter), "◉".to_string())
        };
        let mut node = Node::with_shape(id.clone(), label, NodeShape::Circle);
        node.subgraph = composite.map(String::from);
        graph.nodes.insert(id.clone(), node);
        id
    } else {
        ensure_state_exists(graph, state_ref, composite);
        state_ref.to_string()
    }
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

    #[test]
    fn test_parse_state_ref() {
        assert_eq!(parse_state_ref.parse("[*]").unwrap(), "[*]");
        assert_eq!(parse_state_ref.parse("Idle").unwrap(), "Idle");
        assert_eq!(parse_state_ref.parse("state_1").unwrap(), "state_1");
    }
}
