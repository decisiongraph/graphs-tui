use crate::error::MermaidError;
use crate::types::{Direction, Edge, EdgeStyle, Graph, Node, NodeId, NodeShape, NodeStyle, Subgraph};

/// Parse mermaid flowchart syntax into a Graph
pub fn parse_mermaid(input: &str) -> Result<Graph, MermaidError> {
    let lines: Vec<&str> = input
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with("%%"))
        .collect();

    if lines.is_empty() {
        return Err(MermaidError::EmptyInput);
    }

    // Parse first line: flowchart/graph DIRECTION
    let first_line = lines[0];
    let direction = parse_flowchart_header(first_line)?;

    let mut graph = Graph::new(direction);
    let mut current_subgraph: Option<String> = None;

    // Parse remaining lines
    for (i, line) in lines.iter().enumerate().skip(1) {
        // Check for classDef: classDef name color:#hex
        if line.to_lowercase().starts_with("classdef ") {
            if let Some((name, style)) = parse_class_def(line) {
                graph.style_classes.insert(name, style);
            }
            continue;
        }

        // Check for class assignment: class A,B,C className
        if line.to_lowercase().starts_with("class ") {
            parse_class_assignment(&mut graph, line);
            continue;
        }

        // Check for subgraph start
        if line.to_lowercase().starts_with("subgraph") {
            let subgraph = parse_subgraph_header(line, i + 1)?;
            current_subgraph = Some(subgraph.id.clone());
            graph.subgraphs.push(subgraph);
            continue;
        }

        // Check for subgraph end
        if line.to_lowercase() == "end" {
            current_subgraph = None;
            continue;
        }

        parse_line(&mut graph, line, i + 1, current_subgraph.as_deref())?;
    }

    Ok(graph)
}

/// Parse the flowchart header line
fn parse_flowchart_header(line: &str) -> Result<Direction, MermaidError> {
    let line_lower = line.to_lowercase();
    if !line_lower.starts_with("flowchart") && !line_lower.starts_with("graph") {
        return Err(MermaidError::ParseError {
            line: 1,
            message: "Unsupported diagram type or missing direction".to_string(),
            suggestion: Some("Use 'flowchart LR', 'graph TD', etc.".to_string()),
        });
    }

    // Extract direction part
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 2 {
        return Err(MermaidError::ParseError {
            line: 1,
            message: "Missing direction".to_string(),
            suggestion: Some("Add direction like 'flowchart LR'".to_string()),
        });
    }

    Direction::parse(parts[1]).ok_or_else(|| MermaidError::ParseError {
        line: 1,
        message: format!("Invalid direction: {}", parts[1]),
        suggestion: Some("Use LR, RL, TB, TD, or BT".to_string()),
    })
}

/// Parse subgraph header: subgraph ID [Label]
fn parse_subgraph_header(line: &str, line_num: usize) -> Result<Subgraph, MermaidError> {
    let rest = line.strip_prefix("subgraph").unwrap_or(line).trim();

    // Check for label in brackets: subgraph ID [Label]
    if let Some(bracket_start) = rest.find('[') {
        let id = rest[..bracket_start].trim();
        if let Some(bracket_end) = rest.rfind(']') {
            let label = &rest[bracket_start + 1..bracket_end];
            return Ok(Subgraph::new(id.to_string(), label.to_string()));
        }
    }

    // Just ID, use as label too
    if !rest.is_empty() && is_valid_id(rest.split_whitespace().next().unwrap_or(rest)) {
        let id = rest.split_whitespace().next().unwrap_or(rest);
        return Ok(Subgraph::new(id.to_string(), id.to_string()));
    }

    Err(MermaidError::ParseError {
        line: line_num,
        message: "Invalid subgraph syntax".to_string(),
        suggestion: Some("Use 'subgraph ID [Label]'".to_string()),
    })
}

/// Edge pattern with style
struct EdgePattern {
    pattern: &'static str,
    style: EdgeStyle,
}

const EDGE_PATTERNS: &[EdgePattern] = &[
    // Order matters - check longer/more specific patterns first
    EdgePattern {
        pattern: "-.->",
        style: EdgeStyle::DottedArrow,
    },
    EdgePattern {
        pattern: "-.-",
        style: EdgeStyle::DottedLine,
    },
    EdgePattern {
        pattern: "==>",
        style: EdgeStyle::ThickArrow,
    },
    EdgePattern {
        pattern: "===",
        style: EdgeStyle::ThickLine,
    },
    EdgePattern {
        pattern: "-->",
        style: EdgeStyle::Arrow,
    },
    EdgePattern {
        pattern: "---",
        style: EdgeStyle::Line,
    },
];

/// Find edge pattern in line and return (pattern, style)
fn find_edge_pattern(line: &str) -> Option<(&'static str, EdgeStyle)> {
    for ep in EDGE_PATTERNS {
        if line.contains(ep.pattern) {
            return Some((ep.pattern, ep.style));
        }
    }
    None
}

/// Parse a single line (node declaration or edge)
fn parse_line(
    graph: &mut Graph,
    line: &str,
    line_num: usize,
    current_subgraph: Option<&str>,
) -> Result<(), MermaidError> {
    // Find which edge pattern is used
    if let Some((pattern, style)) = find_edge_pattern(line) {
        // Split by the edge pattern
        let segments: Vec<&str> = line.split(pattern).map(|s| s.trim()).collect();

        if segments.len() > 1 {
            let mut prev_ids: Vec<NodeId> = Vec::new();
            let mut pending_edge_label: Option<String> = None;

            for segment in segments {
                // Check if segment starts with edge label: |label| Node
                let (edge_label, node_part) = parse_edge_label_prefix(segment);

                // Use edge label from this segment or pending from previous
                let current_edge_label = edge_label.or(pending_edge_label.take());

                // Check if segment ends with edge label for next edge: Node |label|
                let (node_segment, next_edge_label) = parse_edge_label_suffix(node_part);
                pending_edge_label = next_edge_label;

                if node_segment.is_empty() {
                    continue;
                }

                // Parse multi-target: A & B & C
                let targets = parse_multi_target(node_segment);
                let mut current_ids: Vec<NodeId> = Vec::new();

                for target in targets {
                    let target = target.trim();
                    if target.is_empty() {
                        continue;
                    }

                    let (id, node_label, shape, style_class) = parse_node_segment(target, line_num)?;

                    // Add or update node
                    add_or_update_node(graph, &id, node_label, shape, current_subgraph, style_class);

                    // Add edges from all previous nodes
                    for from_id in &prev_ids {
                        graph.edges.push(Edge {
                            from: from_id.clone(),
                            to: id.clone(),
                            label: current_edge_label.clone(),
                            style,
                        });
                    }

                    current_ids.push(id);
                }

                prev_ids = current_ids;
            }
        }
    } else {
        // Single node declaration
        let (id, label, shape, style_class) = parse_node_segment(line, line_num)?;
        add_or_update_node(graph, &id, label, shape, current_subgraph, style_class);
    }

    Ok(())
}

/// Parse multi-target syntax: "A & B & C" -> vec!["A", "B", "C"]
fn parse_multi_target(segment: &str) -> Vec<&str> {
    // Only split by & if it's surrounded by whitespace (to avoid splitting inside labels)
    if segment.contains(" & ") {
        segment.split(" & ").collect()
    } else {
        vec![segment]
    }
}

/// Add a node to the graph or update it if it exists
fn add_or_update_node(
    graph: &mut Graph,
    id: &str,
    label: Option<String>,
    shape: NodeShape,
    current_subgraph: Option<&str>,
    style_class: Option<String>,
) {
    if !graph.nodes.contains_key(id) {
        let node_label = label.unwrap_or_else(|| id.to_string());
        let mut node = Node::with_shape(id.to_string(), node_label, shape);
        node.subgraph = current_subgraph.map(|s| s.to_string());
        node.style_class = style_class;
        graph.nodes.insert(id.to_string(), node);

        // Add to subgraph's node list
        if let Some(sg_id) = current_subgraph {
            if let Some(sg) = graph.subgraphs.iter_mut().find(|s| s.id == sg_id) {
                sg.nodes.push(id.to_string());
            }
        }
    } else {
        if let Some(node) = graph.nodes.get_mut(id) {
            if let Some(lbl) = label {
                node.label = lbl;
                node.shape = shape;
            }
            if style_class.is_some() {
                node.style_class = style_class;
            }
        }
    }
}

/// Parse edge label prefix: |label| Node -> (Some(label), "Node")
fn parse_edge_label_prefix(segment: &str) -> (Option<String>, &str) {
    let segment = segment.trim();
    if let Some(stripped) = segment.strip_prefix('|') {
        if let Some(end_pipe) = stripped.find('|') {
            let label = stripped[..end_pipe].to_string();
            let rest = stripped[end_pipe + 1..].trim();
            return (Some(label), rest);
        }
    }
    (None, segment)
}

/// Parse edge label suffix: Node |label| -> ("Node", Some(label))
fn parse_edge_label_suffix(segment: &str) -> (&str, Option<String>) {
    let segment = segment.trim();
    // Look for trailing |label| pattern
    if let Some(start_pipe) = segment.rfind('|') {
        // Check if there's a matching pipe before it
        let before = &segment[..start_pipe];
        if let Some(open_pipe) = before.rfind('|') {
            // Check that the node part doesn't contain the pipes (i.e., not inside brackets)
            let node_part = &segment[..open_pipe].trim();
            let label = segment[open_pipe + 1..start_pipe].to_string();
            // Only extract if there's actual node content before
            if !node_part.is_empty() && !node_part.ends_with('[') {
                return (node_part, Some(label));
            }
        }
    }
    (segment, None)
}

/// Parse a node segment and return (id, label, shape, style_class)
/// Supports many mermaid shapes including hexagon, parallelogram, trapezoid
/// Also handles inline class syntax: A:::className
fn parse_node_segment(
    segment: &str,
    line_num: usize,
) -> Result<(NodeId, Option<String>, NodeShape, Option<String>), MermaidError> {
    let segment = segment.trim();

    // Check for inline class syntax: A:::className or A[Label]:::className
    let (segment, style_class) = if let Some(idx) = segment.find(":::") {
        let class_name = segment[idx + 3..].trim().to_string();
        let node_part = segment[..idx].trim();
        (node_part, Some(class_name))
    } else {
        (segment, None)
    };

    // Try each shape pattern
    // Order matters: check longer/more specific patterns first

    // Hexagon: {{Label}}
    if let Some(result) = try_parse_shape(segment, "{{", "}}", NodeShape::Hexagon) {
        return validate_node_result(result, segment, line_num, style_class);
    }

    // Circle: ((Label))
    if let Some(result) = try_parse_shape(segment, "((", "))", NodeShape::Circle) {
        return validate_node_result(result, segment, line_num, style_class);
    }

    // Cylinder/Database: [(Label)]
    if let Some(result) = try_parse_shape(segment, "[(", ")]", NodeShape::Cylinder) {
        return validate_node_result(result, segment, line_num, style_class);
    }

    // Stadium: ([Label])
    if let Some(result) = try_parse_shape(segment, "([", "])", NodeShape::Stadium) {
        return validate_node_result(result, segment, line_num, style_class);
    }

    // Subroutine: [[Label]]
    if let Some(result) = try_parse_shape(segment, "[[", "]]", NodeShape::Subroutine) {
        return validate_node_result(result, segment, line_num, style_class);
    }

    // Trapezoid: [/Label\]
    if let Some(result) = try_parse_shape(segment, "[/", "\\]", NodeShape::Trapezoid) {
        return validate_node_result(result, segment, line_num, style_class);
    }

    // Trapezoid Alt: [\Label/]
    if let Some(result) = try_parse_shape(segment, "[\\", "/]", NodeShape::TrapezoidAlt) {
        return validate_node_result(result, segment, line_num, style_class);
    }

    // Parallelogram: [/Label/]
    if let Some(result) = try_parse_shape(segment, "[/", "/]", NodeShape::Parallelogram) {
        return validate_node_result(result, segment, line_num, style_class);
    }

    // Parallelogram Alt: [\Label\]
    if let Some(result) = try_parse_shape(segment, "[\\", "\\]", NodeShape::ParallelogramAlt) {
        return validate_node_result(result, segment, line_num, style_class);
    }

    // Diamond: {Label}
    if let Some(result) = try_parse_shape(segment, "{", "}", NodeShape::Diamond) {
        return validate_node_result(result, segment, line_num, style_class);
    }

    // Rounded: (Label)
    if let Some(result) = try_parse_shape(segment, "(", ")", NodeShape::Rounded) {
        return validate_node_result(result, segment, line_num, style_class);
    }

    // Rectangle: [Label]
    if let Some(result) = try_parse_shape(segment, "[", "]", NodeShape::Rectangle) {
        return validate_node_result(result, segment, line_num, style_class);
    }

    // Just an ID with no shape
    if is_valid_id(segment) {
        return Ok((segment.to_string(), None, NodeShape::Rectangle, style_class));
    }

    Err(MermaidError::ParseError {
        line: line_num,
        message: format!("Invalid syntax: \"{}\"", segment),
        suggestion: Some("Supported: [Label], (Label), ((Label)), {{Label}}, {Label}, [(Label)], [/Label/], etc.".to_string()),
    })
}

/// Try to parse a node with given delimiters
fn try_parse_shape(
    segment: &str,
    open: &str,
    close: &str,
    shape: NodeShape,
) -> Option<(String, String, NodeShape)> {
    if let Some(start) = segment.find(open) {
        let id = &segment[..start];
        if let Some(end) = segment.rfind(close) {
            if end > start + open.len() {
                let label = &segment[start + open.len()..end];
                // Handle <br/> line breaks - replace with space for now
                let label = label.replace("<br/>", " ").replace("<br>", " ");
                return Some((id.to_string(), label, shape));
            }
        }
    }
    None
}

/// Validate the parsed node result
fn validate_node_result(
    result: (String, String, NodeShape),
    segment: &str,
    line_num: usize,
    style_class: Option<String>,
) -> Result<(NodeId, Option<String>, NodeShape, Option<String>), MermaidError> {
    let (id, label, shape) = result;
    if !is_valid_id(&id) {
        return Err(MermaidError::ParseError {
            line: line_num,
            message: format!("Invalid node ID in: \"{}\"", segment),
            suggestion: Some("Node ID must be alphanumeric".to_string()),
        });
    }
    Ok((id, Some(label), shape, style_class))
}

/// Check if string is a valid node ID (alphanumeric + underscore)
fn is_valid_id(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_alphanumeric() || c == '_')
}

/// Parse classDef line: classDef name color:#hex or classDef name fill:#hex,stroke:#hex
fn parse_class_def(line: &str) -> Option<(String, NodeStyle)> {
    let rest = line
        .strip_prefix("classDef ")
        .or_else(|| line.strip_prefix("classDef"))?
        .trim();

    // Split into name and properties
    let parts: Vec<&str> = rest.splitn(2, char::is_whitespace).collect();
    if parts.is_empty() {
        return None;
    }

    let name = parts[0].to_string();
    let props = parts.get(1).unwrap_or(&"");

    // Parse color from properties (look for color:#hex or fill:#hex)
    let color = extract_color(props);

    Some((name, NodeStyle { color }))
}

/// Extract color value from classDef properties
fn extract_color(props: &str) -> Option<String> {
    for part in props.split(',') {
        let part = part.trim();
        if let Some(color) = part.strip_prefix("color:") {
            return Some(hex_to_ansi(color.trim()));
        }
        if let Some(color) = part.strip_prefix("fill:") {
            // Use fill as background (we'll use it for foreground in terminal)
            return Some(hex_to_ansi(color.trim()));
        }
    }
    None
}

/// Convert hex color to ANSI escape code
fn hex_to_ansi(hex: &str) -> String {
    let hex = hex.trim_start_matches('#');
    if hex.len() >= 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&hex[0..2], 16),
            u8::from_str_radix(&hex[2..4], 16),
            u8::from_str_radix(&hex[4..6], 16),
        ) {
            // Use 24-bit ANSI color: \x1b[38;2;R;G;Bm
            return format!("\x1b[38;2;{};{};{}m", r, g, b);
        }
    }
    // Return empty string if invalid
    String::new()
}

/// Parse class assignment: class A,B,C className
fn parse_class_assignment(graph: &mut Graph, line: &str) {
    let rest = line
        .strip_prefix("class ")
        .or_else(|| line.strip_prefix("class"))
        .unwrap_or(line)
        .trim();

    // Split into node list and class name
    let parts: Vec<&str> = rest.rsplitn(2, char::is_whitespace).collect();
    if parts.len() != 2 {
        return;
    }

    let class_name = parts[0].trim();
    let node_list = parts[1].trim();

    // Apply class to each node
    for node_id in node_list.split(',') {
        let node_id = node_id.trim();
        if let Some(node) = graph.nodes.get_mut(node_id) {
            node.style_class = Some(class_name.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_lr() {
        let input = "flowchart LR\nA --> B";
        let graph = parse_mermaid(input).unwrap();
        assert_eq!(graph.direction, Direction::LR);
        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.edges.len(), 1);
    }

    #[test]
    fn test_parse_graph_td() {
        let input = "graph TD\nA --> B";
        let graph = parse_mermaid(input).unwrap();
        assert_eq!(graph.direction, Direction::TB);
        assert_eq!(graph.nodes.len(), 2);
    }

    #[test]
    fn test_parse_with_labels() {
        let input = "flowchart TB\nA[Start] --> B[End]";
        let graph = parse_mermaid(input).unwrap();
        assert_eq!(graph.nodes.get("A").unwrap().label, "Start");
        assert_eq!(graph.nodes.get("B").unwrap().label, "End");
    }

    #[test]
    fn test_parse_chain() {
        let input = "flowchart LR\nA --> B --> C --> D";
        let graph = parse_mermaid(input).unwrap();
        assert_eq!(graph.edges.len(), 3);
        assert_eq!(
            graph.edges[0],
            Edge {
                from: "A".to_string(),
                to: "B".to_string(),
                label: None,
                style: EdgeStyle::Arrow
            }
        );
        assert_eq!(
            graph.edges[1],
            Edge {
                from: "B".to_string(),
                to: "C".to_string(),
                label: None,
                style: EdgeStyle::Arrow
            }
        );
        assert_eq!(
            graph.edges[2],
            Edge {
                from: "C".to_string(),
                to: "D".to_string(),
                label: None,
                style: EdgeStyle::Arrow
            }
        );
    }

    #[test]
    fn test_parse_edge_labels() {
        let input = "flowchart LR\nA -->|sends| B";
        let graph = parse_mermaid(input).unwrap();
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.edges[0].label, Some("sends".to_string()));
    }

    #[test]
    fn test_parse_edge_labels_chain() {
        let input = "flowchart LR\nA -->|first| B -->|second| C";
        let graph = parse_mermaid(input).unwrap();
        assert_eq!(graph.edges.len(), 2);
        assert_eq!(graph.edges[0].label, Some("first".to_string()));
        assert_eq!(graph.edges[1].label, Some("second".to_string()));
    }

    #[test]
    fn test_parse_comments() {
        let input = "flowchart LR\n%% comment\nA --> B";
        let graph = parse_mermaid(input).unwrap();
        assert_eq!(graph.nodes.len(), 2);
    }

    #[test]
    fn test_parse_empty_input() {
        let result = parse_mermaid("");
        assert!(matches!(result, Err(MermaidError::EmptyInput)));
    }

    #[test]
    fn test_parse_invalid_diagram() {
        let result = parse_mermaid("sequenceDiagram\nA->B");
        assert!(matches!(result, Err(MermaidError::ParseError { .. })));
    }

    #[test]
    fn test_parse_label_update() {
        let input = "flowchart LR\nA\nA[Label A]\nA --> B";
        let graph = parse_mermaid(input).unwrap();
        assert_eq!(graph.nodes.get("A").unwrap().label, "Label A");
    }

    #[test]
    fn test_parse_labels_with_spaces() {
        let input = "flowchart LR\nA[Start Here] --> B[Wait... what?]";
        let graph = parse_mermaid(input).unwrap();
        assert_eq!(graph.nodes.get("A").unwrap().label, "Start Here");
        assert_eq!(graph.nodes.get("B").unwrap().label, "Wait... what?");
    }

    #[test]
    fn test_parse_circle_shape() {
        let input = "flowchart LR\nA((Circle))";
        let graph = parse_mermaid(input).unwrap();
        assert_eq!(graph.nodes.get("A").unwrap().shape, NodeShape::Circle);
        assert_eq!(graph.nodes.get("A").unwrap().label, "Circle");
    }

    #[test]
    fn test_parse_diamond_shape() {
        let input = "flowchart LR\nA{Decision}";
        let graph = parse_mermaid(input).unwrap();
        assert_eq!(graph.nodes.get("A").unwrap().shape, NodeShape::Diamond);
    }

    #[test]
    fn test_parse_cylinder_shape() {
        let input = "flowchart LR\nDB[(Database)]";
        let graph = parse_mermaid(input).unwrap();
        assert_eq!(graph.nodes.get("DB").unwrap().shape, NodeShape::Cylinder);
    }

    #[test]
    fn test_parse_rounded_shape() {
        let input = "flowchart LR\nA(Rounded)";
        let graph = parse_mermaid(input).unwrap();
        assert_eq!(graph.nodes.get("A").unwrap().shape, NodeShape::Rounded);
    }

    #[test]
    fn test_parse_stadium_shape() {
        let input = "flowchart LR\nA([Stadium])";
        let graph = parse_mermaid(input).unwrap();
        assert_eq!(graph.nodes.get("A").unwrap().shape, NodeShape::Stadium);
    }

    #[test]
    fn test_parse_subroutine_shape() {
        let input = "flowchart LR\nA[[Subroutine]]";
        let graph = parse_mermaid(input).unwrap();
        assert_eq!(graph.nodes.get("A").unwrap().shape, NodeShape::Subroutine);
    }

    #[test]
    fn test_parse_subgraph() {
        let input =
            "flowchart TB\nsubgraph Backend [Backend Services]\nA[API]\nB[DB]\nend\nA --> B";
        let graph = parse_mermaid(input).unwrap();
        assert_eq!(graph.subgraphs.len(), 1);
        assert_eq!(graph.subgraphs[0].id, "Backend");
        assert_eq!(graph.subgraphs[0].label, "Backend Services");
        assert_eq!(graph.subgraphs[0].nodes.len(), 2);
        assert_eq!(
            graph.nodes.get("A").unwrap().subgraph,
            Some("Backend".to_string())
        );
    }

    #[test]
    fn test_parse_br_tags() {
        let input = "flowchart LR\nA[Line1<br/>Line2]";
        let graph = parse_mermaid(input).unwrap();
        assert_eq!(graph.nodes.get("A").unwrap().label, "Line1 Line2");
    }

    // ===== NEW SHAPE TESTS (TDD) =====

    #[test]
    fn test_parse_hexagon_shape() {
        let input = "flowchart LR\nA{{Hexagon}}";
        let graph = parse_mermaid(input).unwrap();
        assert_eq!(graph.nodes.get("A").unwrap().shape, NodeShape::Hexagon);
        assert_eq!(graph.nodes.get("A").unwrap().label, "Hexagon");
    }

    #[test]
    fn test_parse_parallelogram_shape() {
        let input = "flowchart LR\nA[/Parallelogram/]";
        let graph = parse_mermaid(input).unwrap();
        assert_eq!(
            graph.nodes.get("A").unwrap().shape,
            NodeShape::Parallelogram
        );
        assert_eq!(graph.nodes.get("A").unwrap().label, "Parallelogram");
    }

    #[test]
    fn test_parse_parallelogram_alt_shape() {
        let input = "flowchart LR\nA[\\Parallelogram Alt\\]";
        let graph = parse_mermaid(input).unwrap();
        assert_eq!(
            graph.nodes.get("A").unwrap().shape,
            NodeShape::ParallelogramAlt
        );
    }

    #[test]
    fn test_parse_trapezoid_shape() {
        let input = "flowchart LR\nA[/Trapezoid\\]";
        let graph = parse_mermaid(input).unwrap();
        assert_eq!(graph.nodes.get("A").unwrap().shape, NodeShape::Trapezoid);
    }

    #[test]
    fn test_parse_trapezoid_alt_shape() {
        let input = "flowchart LR\nA[\\Trapezoid Alt/]";
        let graph = parse_mermaid(input).unwrap();
        assert_eq!(graph.nodes.get("A").unwrap().shape, NodeShape::TrapezoidAlt);
    }

    // ===== NEW EDGE STYLE TESTS (TDD) =====

    #[test]
    fn test_parse_solid_line() {
        let input = "flowchart LR\nA --- B";
        let graph = parse_mermaid(input).unwrap();
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.edges[0].style, EdgeStyle::Line);
    }

    #[test]
    fn test_parse_dotted_arrow() {
        let input = "flowchart LR\nA -.-> B";
        let graph = parse_mermaid(input).unwrap();
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.edges[0].style, EdgeStyle::DottedArrow);
    }

    #[test]
    fn test_parse_dotted_line() {
        let input = "flowchart LR\nA -.- B";
        let graph = parse_mermaid(input).unwrap();
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.edges[0].style, EdgeStyle::DottedLine);
    }

    #[test]
    fn test_parse_thick_arrow() {
        let input = "flowchart LR\nA ==> B";
        let graph = parse_mermaid(input).unwrap();
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.edges[0].style, EdgeStyle::ThickArrow);
    }

    #[test]
    fn test_parse_thick_line() {
        let input = "flowchart LR\nA === B";
        let graph = parse_mermaid(input).unwrap();
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.edges[0].style, EdgeStyle::ThickLine);
    }

    #[test]
    fn test_parse_dotted_arrow_with_label() {
        let input = "flowchart LR\nA -.->|async| B";
        let graph = parse_mermaid(input).unwrap();
        assert_eq!(graph.edges[0].style, EdgeStyle::DottedArrow);
        assert_eq!(graph.edges[0].label, Some("async".to_string()));
    }

    // ===== MULTI-TARGET EDGE TESTS =====

    #[test]
    fn test_parse_multi_target_edges() {
        let input = "flowchart LR\nA --> B & C & D";
        let graph = parse_mermaid(input).unwrap();
        assert_eq!(graph.nodes.len(), 4);
        assert_eq!(graph.edges.len(), 3);
        // A -> B, A -> C, A -> D
        assert_eq!(graph.edges[0].from, "A");
        assert_eq!(graph.edges[0].to, "B");
        assert_eq!(graph.edges[1].from, "A");
        assert_eq!(graph.edges[1].to, "C");
        assert_eq!(graph.edges[2].from, "A");
        assert_eq!(graph.edges[2].to, "D");
    }

    #[test]
    fn test_parse_multi_target_with_labels() {
        let input = "flowchart LR\nA[Source] --> B[Target1] & C[Target2]";
        let graph = parse_mermaid(input).unwrap();
        assert_eq!(graph.nodes.get("A").unwrap().label, "Source");
        assert_eq!(graph.nodes.get("B").unwrap().label, "Target1");
        assert_eq!(graph.nodes.get("C").unwrap().label, "Target2");
        assert_eq!(graph.edges.len(), 2);
    }

    #[test]
    fn test_parse_multi_source_to_multi_target() {
        let input = "flowchart LR\nA & B --> C & D";
        let graph = parse_mermaid(input).unwrap();
        assert_eq!(graph.nodes.len(), 4);
        // A->C, A->D, B->C, B->D = 4 edges
        assert_eq!(graph.edges.len(), 4);
    }

    // ===== STYLE CLASS TESTS =====

    #[test]
    fn test_parse_class_def() {
        let input = "flowchart LR\nclassDef red color:#ff0000\nA --> B";
        let graph = parse_mermaid(input).unwrap();
        assert!(graph.style_classes.contains_key("red"));
        let style = graph.style_classes.get("red").unwrap();
        assert!(style.color.is_some());
    }

    #[test]
    fn test_parse_class_assignment() {
        let input = "flowchart LR\nclassDef red color:#ff0000\nA --> B\nclass A red";
        let graph = parse_mermaid(input).unwrap();
        assert_eq!(graph.nodes.get("A").unwrap().style_class, Some("red".to_string()));
    }

    #[test]
    fn test_parse_inline_class() {
        let input = "flowchart LR\nA[Label]:::red --> B";
        let graph = parse_mermaid(input).unwrap();
        assert_eq!(graph.nodes.get("A").unwrap().style_class, Some("red".to_string()));
        assert_eq!(graph.nodes.get("A").unwrap().label, "Label");
    }
}
