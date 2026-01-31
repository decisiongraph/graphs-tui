//! D2 diagram language parser
//!
//! D2 syntax:
//! - Shapes: `id` or `id: "Label"`
//! - Connections: `->`, `<-`, `<->`, `--`
//! - Shape types: `id.shape: circle`
//! - Containers: `parent { child }`
//! - Edge labels: `A -> B: "label"`

use crate::error::MermaidError;
use crate::types::{Direction, Edge, EdgeStyle, Graph, Node, NodeId, NodeShape, Subgraph};

/// Parse D2 diagram syntax into a Graph
pub fn parse_d2(input: &str) -> Result<Graph, MermaidError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(MermaidError::EmptyInput);
    }

    // D2 doesn't have an explicit direction, default to TB (top-bottom)
    let mut graph = Graph::new(Direction::TB);
    let mut current_subgraph: Option<String> = None;
    let mut brace_depth = 0;

    for line in trimmed.lines() {
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Handle opening braces (container start)
        if line.ends_with('{') {
            let container_def = line.trim_end_matches('{').trim();
            if !container_def.is_empty() {
                let (id, label) = parse_d2_label(container_def);
                let sg = Subgraph::new(id.clone(), label);
                graph.subgraphs.push(sg);
                current_subgraph = Some(id);
            }
            brace_depth += 1;
            continue;
        }

        // Handle closing braces (container end)
        if line == "}" {
            brace_depth -= 1;
            if brace_depth == 0 {
                current_subgraph = None;
            }
            continue;
        }

        // Try to parse as connection
        if let Some((from, to, style, label)) = parse_d2_connection(line) {
            // Ensure nodes exist
            ensure_node_exists(&mut graph, &from, current_subgraph.as_deref());
            ensure_node_exists(&mut graph, &to, current_subgraph.as_deref());

            graph.edges.push(Edge {
                from,
                to,
                label,
                style,
            });
            continue;
        }

        // Try to parse as shape property (id.shape: type)
        if let Some((id, shape)) = parse_shape_property(line) {
            if let Some(node) = graph.nodes.get_mut(&id) {
                node.shape = shape;
            } else {
                let mut node = Node::with_shape(id.clone(), id.clone(), shape);
                node.subgraph = current_subgraph.clone();
                graph.nodes.insert(id, node);
            }
            continue;
        }

        // Parse as node declaration
        let (id, label) = parse_d2_label(line);
        if !id.is_empty() {
            use std::collections::hash_map::Entry;
            match graph.nodes.entry(id) {
                Entry::Occupied(mut e) => {
                    e.get_mut().label = label;
                }
                Entry::Vacant(e) => {
                    let mut node = Node::new(e.key().clone(), label);
                    node.subgraph = current_subgraph.clone();
                    e.insert(node);
                }
            }
        }
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

    Ok(graph)
}

/// Ensure a node exists in the graph
fn ensure_node_exists(graph: &mut Graph, id: &str, subgraph: Option<&str>) {
    graph.nodes.entry(id.to_string()).or_insert_with(|| {
        let mut node = Node::new(id.to_string(), id.to_string());
        node.subgraph = subgraph.map(String::from);
        node
    });
}

/// Parse D2 label syntax: `id: "Label"` or `id: Label` or just `id`
fn parse_d2_label(s: &str) -> (String, String) {
    // Handle semicolon-separated declarations
    let s = if let Some(idx) = s.find(';') {
        s[..idx].trim()
    } else {
        s
    };

    if let Some(colon_idx) = s.find(':') {
        let id = s[..colon_idx].trim().to_string();
        let label = s[colon_idx + 1..]
            .trim()
            .trim_matches('"')
            .trim_matches('\'')
            .to_string();
        let final_label = if label.is_empty() { id.clone() } else { label };
        (id, final_label)
    } else {
        let id = s.trim().to_string();
        (id.clone(), id)
    }
}

/// Parse D2 connection syntax
fn parse_d2_connection(line: &str) -> Option<(NodeId, NodeId, EdgeStyle, Option<String>)> {
    // Connection patterns in order of precedence
    let patterns = [
        ("<->", EdgeStyle::Arrow, true), // bidirectional
        ("->", EdgeStyle::Arrow, false), // forward arrow
        ("<-", EdgeStyle::Arrow, false), // backward arrow (we'll swap)
        ("--", EdgeStyle::Line, false),  // simple line
    ];

    for (pattern, style, _is_bidirectional) in patterns {
        if let Some(idx) = line.find(pattern) {
            let left = line[..idx].trim();
            let right_part = line[idx + pattern.len()..].trim();

            // Check if there's a label after the connection
            let (to, label) = if let Some(colon_idx) = right_part.find(':') {
                let to_id = right_part[..colon_idx].trim().to_string();
                let lbl = right_part[colon_idx + 1..]
                    .trim()
                    .trim_matches('"')
                    .trim_matches('\'')
                    .to_string();
                (to_id, Some(lbl))
            } else {
                (right_part.to_string(), None)
            };

            let from = left.to_string();

            // Handle backward arrow by swapping
            if pattern == "<-" {
                return Some((to, from, style, label));
            }

            return Some((from, to, style, label));
        }
    }

    None
}

/// Parse shape property: `id.shape: type`
fn parse_shape_property(line: &str) -> Option<(NodeId, NodeShape)> {
    if !line.contains(".shape:") {
        return None;
    }

    let parts: Vec<&str> = line.splitn(2, ".shape:").collect();
    if parts.len() != 2 {
        return None;
    }

    let id = parts[0].trim().to_string();
    let shape_str = parts[1].trim().to_lowercase();

    let shape = match shape_str.as_str() {
        "rectangle" | "rect" => NodeShape::Rectangle,
        "square" => NodeShape::Rectangle,
        "circle" => NodeShape::Circle,
        "oval" | "ellipse" => NodeShape::Rounded,
        "diamond" => NodeShape::Diamond,
        "cylinder" | "queue" => NodeShape::Cylinder,
        "hexagon" => NodeShape::Hexagon,
        "parallelogram" => NodeShape::Parallelogram,
        "document" | "page" => NodeShape::Rectangle,
        "package" | "step" => NodeShape::Rectangle,
        "cloud" => NodeShape::Rounded,
        "person" => NodeShape::Circle, // Approximate with circle
        _ => NodeShape::Rectangle,
    };

    Some((id, shape))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_d2_simple() {
        let input = "A -> B";
        let graph = parse_d2(input).unwrap();
        assert!(graph.nodes.contains_key("A"));
        assert!(graph.nodes.contains_key("B"));
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.edges[0].from, "A");
        assert_eq!(graph.edges[0].to, "B");
    }

    #[test]
    fn test_parse_d2_with_labels() {
        let input = r#"
server: "Web Server"
db: Database
server -> db
"#;
        let graph = parse_d2(input).unwrap();
        assert_eq!(graph.nodes.get("server").unwrap().label, "Web Server");
        assert_eq!(graph.nodes.get("db").unwrap().label, "Database");
    }

    #[test]
    fn test_parse_d2_edge_label() {
        let input = "A -> B: \"HTTP request\"";
        let graph = parse_d2(input).unwrap();
        assert_eq!(graph.edges[0].label, Some("HTTP request".to_string()));
    }

    #[test]
    fn test_parse_d2_chain() {
        let input = r#"
A -> B
B -> C
C -> D
"#;
        let graph = parse_d2(input).unwrap();
        assert_eq!(graph.edges.len(), 3);
    }

    #[test]
    fn test_parse_d2_backward_arrow() {
        let input = "A <- B";
        let graph = parse_d2(input).unwrap();
        assert_eq!(graph.edges[0].from, "B");
        assert_eq!(graph.edges[0].to, "A");
    }

    #[test]
    fn test_parse_d2_line() {
        let input = "A -- B";
        let graph = parse_d2(input).unwrap();
        assert!(matches!(graph.edges[0].style, EdgeStyle::Line));
    }

    #[test]
    fn test_parse_d2_shape_property() {
        let input = r#"
db: Database
db.shape: cylinder
"#;
        let graph = parse_d2(input).unwrap();
        assert!(matches!(
            graph.nodes.get("db").unwrap().shape,
            NodeShape::Cylinder
        ));
    }

    #[test]
    fn test_parse_d2_container() {
        let input = r#"
backend {
    api: "API Server"
    db: Database
}
api -> db
"#;
        let graph = parse_d2(input).unwrap();
        assert_eq!(graph.subgraphs.len(), 1);
        assert_eq!(graph.subgraphs[0].id, "backend");
        assert_eq!(
            graph.nodes.get("api").unwrap().subgraph,
            Some("backend".to_string())
        );
    }

    #[test]
    fn test_parse_d2_comments() {
        let input = r#"
# This is a comment
A -> B
"#;
        let graph = parse_d2(input).unwrap();
        assert_eq!(graph.edges.len(), 1);
    }

    #[test]
    fn test_parse_d2_empty() {
        let result = parse_d2("");
        assert!(matches!(result, Err(MermaidError::EmptyInput)));
    }
}
