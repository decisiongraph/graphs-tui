use winnow::ascii::{space0, space1, Caseless};
use winnow::combinator::{alt, delimited};
use winnow::error::{ErrMode, ParserError};
use winnow::token::{rest, take_until, take_while};
use winnow::ModalResult;
use winnow::Parser;

use crate::error::MermaidError;
use crate::types::{
    Direction, Edge, EdgeStyle, Graph, Node, NodeId, NodeShape, NodeStyle, Subgraph,
};

/// Content of a single line (after trimming)
#[derive(Debug)]
enum MermaidLine {
    ClassDef {
        name: String,
        style: NodeStyle,
    },
    ClassAssignment {
        class_name: String,
        node_ids: Vec<String>,
    },
    SubgraphStart {
        id: String,
        label: String,
    },
    SubgraphEnd,
    Content(String),
}

// ===== Winnow parsers =====

/// Parse flowchart/graph keyword + direction
fn w_header(input: &mut &str) -> ModalResult<Direction> {
    let _ = alt((Caseless("flowchart"), Caseless("graph"))).parse_next(input)?;
    let _ = space1.parse_next(input)?;
    alt((
        "LR".value(Direction::LR),
        "RL".value(Direction::RL),
        "TB".value(Direction::TB),
        "TD".value(Direction::TB),
        "BT".value(Direction::BT),
    ))
    .parse_next(input)
}

/// Parse classDef: classDef name props...
fn w_classdef(input: &mut &str) -> ModalResult<(String, NodeStyle)> {
    let _ = Caseless("classdef").parse_next(input)?;
    let _ = space1.parse_next(input)?;
    let name: &str = take_while(1.., |c: char| !c.is_whitespace()).parse_next(input)?;
    let _ = space0.parse_next(input)?;
    let props: &str = rest.parse_next(input)?;
    let color = extract_color(props);
    Ok((name.to_string(), NodeStyle { color }))
}

/// Parse class assignment: class A,B,C className
fn w_class_assignment(input: &mut &str) -> ModalResult<(Vec<String>, String)> {
    let _ = Caseless("class").parse_next(input)?;
    let _ = space1.parse_next(input)?;
    let rest_str: &str = rest.parse_next(input)?;
    let parts: Vec<&str> = rest_str.rsplitn(2, char::is_whitespace).collect();
    if parts.len() != 2 {
        return Err(ErrMode::from_input(input));
    }
    let class_name = parts[0].trim().to_string();
    let node_ids = parts[1].split(',').map(|s| s.trim().to_string()).collect();
    Ok((node_ids, class_name))
}

/// Parse subgraph header: subgraph ID [Label] or subgraph ID
fn w_subgraph(input: &mut &str) -> ModalResult<(String, String)> {
    let _ = Caseless("subgraph").parse_next(input)?;
    let _ = space1.parse_next(input)?;
    let rest_str: &str = rest.parse_next(input)?;

    // Check for label in brackets: ID [Label]
    if let Some(bracket_start) = rest_str.find('[') {
        let id = rest_str[..bracket_start].trim();
        if let Some(bracket_end) = rest_str.rfind(']') {
            let label = &rest_str[bracket_start + 1..bracket_end];
            return Ok((id.to_string(), label.to_string()));
        }
    }

    // Just ID, use as label too
    let id = rest_str
        .split_whitespace()
        .next()
        .unwrap_or(rest_str)
        .trim();
    if !id.is_empty() && is_valid_id(id) {
        return Ok((id.to_string(), id.to_string()));
    }
    Err(ErrMode::from_input(input))
}

/// Parse edge label: |label|
fn w_edge_label(input: &mut &str) -> ModalResult<String> {
    delimited('|', take_until(0.., "|"), '|')
        .map(|s: &str| s.to_string())
        .parse_next(input)
}

/// Classify a line into its type
fn classify_line(line: &str) -> Result<MermaidLine, MermaidError> {
    let trimmed = line.trim();

    // Try classdef (must be before class)
    let mut input = trimmed;
    if let Ok((name, style)) = w_classdef(&mut input) {
        return Ok(MermaidLine::ClassDef { name, style });
    }

    // Try class assignment
    input = trimmed;
    if let Ok((node_ids, class_name)) = w_class_assignment(&mut input) {
        return Ok(MermaidLine::ClassAssignment {
            class_name,
            node_ids,
        });
    }

    // Try subgraph end (must check before subgraph start)
    if trimmed.eq_ignore_ascii_case("end") {
        return Ok(MermaidLine::SubgraphEnd);
    }

    // Try subgraph start
    input = trimmed;
    if let Ok((id, label)) = w_subgraph(&mut input) {
        return Ok(MermaidLine::SubgraphStart { id, label });
    }

    // Default: content line (edge or node)
    Ok(MermaidLine::Content(trimmed.to_string()))
}

// ===== Main parse function =====

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

    let direction = parse_flowchart_header(lines[0])?;
    let mut graph = Graph::new(direction);
    let mut current_subgraph: Option<String> = None;

    for (i, line) in lines.iter().enumerate().skip(1) {
        match classify_line(line)? {
            MermaidLine::ClassDef { name, style } => {
                graph.style_classes.insert(name, style);
            }
            MermaidLine::ClassAssignment {
                class_name,
                node_ids,
            } => {
                for node_id in &node_ids {
                    if let Some(node) = graph.nodes.get_mut(node_id.as_str()) {
                        node.style_class = Some(class_name.clone());
                    }
                }
            }
            MermaidLine::SubgraphStart { id, label } => {
                current_subgraph = Some(id.clone());
                graph.subgraphs.push(Subgraph::new(id, label));
            }
            MermaidLine::SubgraphEnd => {
                current_subgraph = None;
            }
            MermaidLine::Content(content) => {
                parse_content_line(&mut graph, &content, i + 1, current_subgraph.as_deref())?;
            }
        }
    }

    Ok(graph)
}

/// Parse the flowchart header line using winnow
fn parse_flowchart_header(line: &str) -> Result<Direction, MermaidError> {
    let mut input = line;
    w_header(&mut input).map_err(|_| MermaidError::ParseError {
        line: 1,
        message: "Unsupported diagram type or missing direction".to_string(),
        suggestion: Some("Use 'flowchart LR', 'graph TD', etc.".to_string()),
    })
}

// ===== Edge patterns =====

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

// ===== Content line parsing =====

/// Parse a content line (node declaration or edge)
fn parse_content_line(
    graph: &mut Graph,
    line: &str,
    line_num: usize,
    current_subgraph: Option<&str>,
) -> Result<(), MermaidError> {
    if let Some((pattern, style)) = find_edge_pattern(line) {
        let segments: Vec<&str> = line.split(pattern).map(|s| s.trim()).collect();

        if segments.len() > 1 {
            let mut prev_ids: Vec<NodeId> = Vec::new();
            let mut pending_edge_label: Option<String> = None;

            for segment in segments {
                let (edge_label, node_part) = extract_edge_label_prefix(segment);
                let current_edge_label = edge_label.or(pending_edge_label.take());
                let (node_segment, next_edge_label) = extract_edge_label_suffix(node_part);
                pending_edge_label = next_edge_label;

                if node_segment.is_empty() {
                    continue;
                }

                let targets = parse_multi_target(node_segment);
                let mut current_ids: Vec<NodeId> = Vec::new();

                for target in targets {
                    let target = target.trim();
                    if target.is_empty() {
                        continue;
                    }

                    let (id, node_label, shape, style_class) =
                        parse_node_segment(target, line_num)?;

                    add_or_update_node(
                        graph,
                        &id,
                        node_label,
                        shape,
                        current_subgraph,
                        style_class,
                    );

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

        if let Some(sg_id) = current_subgraph {
            if let Some(sg) = graph.subgraphs.iter_mut().find(|s| s.id == sg_id) {
                sg.nodes.push(id.to_string());
            }
        }
    } else if let Some(node) = graph.nodes.get_mut(id) {
        if let Some(lbl) = label {
            node.label = lbl;
            node.shape = shape;
        }
        if style_class.is_some() {
            node.style_class = style_class;
        }
    }
}

/// Extract edge label prefix: |label| Node -> (Some(label), "Node")
fn extract_edge_label_prefix(segment: &str) -> (Option<String>, &str) {
    let segment = segment.trim();
    let mut input = segment;
    if let Ok(label) = w_edge_label(&mut input) {
        let rest = input.trim();
        return (Some(label), rest);
    }
    (None, segment)
}

/// Extract edge label suffix: Node |label| -> ("Node", Some(label))
fn extract_edge_label_suffix(segment: &str) -> (&str, Option<String>) {
    let segment = segment.trim();
    // Look for trailing |label| pattern
    if let Some(start_pipe) = segment.rfind('|') {
        let before = &segment[..start_pipe];
        if let Some(open_pipe) = before.rfind('|') {
            let node_part = segment[..open_pipe].trim();
            let label = segment[open_pipe + 1..start_pipe].to_string();
            if !node_part.is_empty() && !node_part.ends_with('[') {
                return (node_part, Some(label));
            }
        }
    }
    (segment, None)
}

// ===== Node segment parsing =====

/// Parse a node segment: ID + optional shape(label) + optional :::class
fn parse_node_segment(
    segment: &str,
    line_num: usize,
) -> Result<(NodeId, Option<String>, NodeShape, Option<String>), MermaidError> {
    let segment = segment.trim();

    // Extract inline class suffix: :::className
    let (segment, style_class) = if let Some(idx) = segment.find(":::") {
        let class_name = segment[idx + 3..].trim().to_string();
        let node_part = segment[..idx].trim();
        (node_part, Some(class_name))
    } else {
        (segment, None)
    };

    // Try each shape pattern (order matters: longer/more specific first)
    let shape_attempts: &[(&str, &str, NodeShape)] = &[
        ("{{", "}}", NodeShape::Hexagon),
        ("((", "))", NodeShape::Circle),
        ("[(", ")]", NodeShape::Cylinder),
        ("([", "])", NodeShape::Stadium),
        ("[[", "]]", NodeShape::Subroutine),
        ("[/", "\\]", NodeShape::Trapezoid),
        ("[\\", "/]", NodeShape::TrapezoidAlt),
        ("[/", "/]", NodeShape::Parallelogram),
        ("[\\", "\\]", NodeShape::ParallelogramAlt),
        ("{", "}", NodeShape::Diamond),
        ("(", ")", NodeShape::Rounded),
        ("[", "]", NodeShape::Rectangle),
    ];

    for &(open, close, shape) in shape_attempts {
        if let Some(result) = try_parse_shape(segment, open, close, shape) {
            return validate_node_result(result, segment, line_num, style_class);
        }
    }

    // Just an ID with no shape
    if is_valid_id(segment) {
        return Ok((segment.to_string(), None, NodeShape::Rectangle, style_class));
    }

    Err(MermaidError::ParseError {
        line: line_num,
        message: format!("Invalid syntax: \"{}\"", segment),
        suggestion: Some(
            "Supported: [Label], (Label), ((Label)), {{Label}}, {Label}, [(Label)], [/Label/], etc."
                .to_string(),
        ),
    })
}

/// Try to parse a node with given delimiters
fn try_parse_shape(
    segment: &str,
    open: &str,
    close: &str,
    shape: NodeShape,
) -> Option<(String, String, NodeShape)> {
    let start = segment.find(open)?;
    let id = &segment[..start];
    let end = segment.rfind(close)?;
    if end > start + open.len() {
        let label = &segment[start + open.len()..end];
        let label = normalize_label(label);
        Some((id.to_string(), label, shape))
    } else {
        None
    }
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

/// Normalize label text (handle <br/> tags as line breaks)
fn normalize_label(label: &str) -> String {
    label.replace("<br/>", "\n").replace("<br>", "\n")
}

/// Check if string is a valid node ID (alphanumeric + underscore)
fn is_valid_id(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_alphanumeric() || c == '_')
}

// ===== Color parsing =====

/// Extract color value from classDef properties
fn extract_color(props: &str) -> Option<String> {
    for part in props.split(',') {
        let part = part.trim();
        if let Some(color) = part.strip_prefix("color:") {
            return Some(hex_to_ansi(color.trim()));
        }
        if let Some(color) = part.strip_prefix("fill:") {
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
            return format!("\x1b[38;2;{};{};{}m", r, g, b);
        }
    }
    String::new()
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
        assert_eq!(graph.nodes.get("A").unwrap().label, "Line1\nLine2");
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
        assert_eq!(
            graph.nodes.get("A").unwrap().style_class,
            Some("red".to_string())
        );
    }

    #[test]
    fn test_parse_inline_class() {
        let input = "flowchart LR\nA[Label]:::red --> B";
        let graph = parse_mermaid(input).unwrap();
        assert_eq!(
            graph.nodes.get("A").unwrap().style_class,
            Some("red".to_string())
        );
        assert_eq!(graph.nodes.get("A").unwrap().label, "Label");
    }
}
