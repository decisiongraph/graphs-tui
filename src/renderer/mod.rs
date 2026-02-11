//! Renderer module for converting graphs to text output

pub mod backend;
mod charset;
mod edges;
mod shapes;
mod subgraph;

use crate::grid::Grid;
use crate::pathfinding::PathGrid;
use crate::types::{DiagramWarning, Graph, Node, RenderOptions};

use charset::{ASCII_CHARS, UNICODE_CHARS};

use edges::draw_edge;
use shapes::draw_node;
use subgraph::{draw_subgraph, protect_subgraph_borders};

/// Build a PathGrid with all nodes marked as obstacles
fn build_path_grid(graph: &Graph, width: usize, height: usize) -> PathGrid {
    let mut path_grid = PathGrid::new(width, height);

    // Mark all nodes as obstacles
    for node in graph.nodes.values() {
        path_grid.block_rect(node.x, node.y, node.width, node.height);
    }

    // Mark subgraph borders as obstacles too
    for sg in &graph.subgraphs {
        if sg.width > 0 && sg.height > 0 {
            // Top border
            path_grid.block_rect(sg.x, sg.y, sg.width, 1);
            // Bottom border
            path_grid.block_rect(sg.x, sg.y + sg.height.saturating_sub(1), sg.width, 1);
            // Left border
            path_grid.block_rect(sg.x, sg.y, 1, sg.height);
            // Right border
            path_grid.block_rect(sg.x + sg.width.saturating_sub(1), sg.y, 1, sg.height);
        }
    }

    path_grid
}

/// Render the graph to a string
pub fn render_graph(
    graph: &Graph,
    options: &RenderOptions,
    warnings: &mut Vec<DiagramWarning>,
) -> String {
    let chars = if options.ascii {
        &ASCII_CHARS
    } else {
        &UNICODE_CHARS
    };

    // Find grid bounds
    let mut max_x = 0;
    let mut max_y = 0;

    // Sort nodes by id for deterministic rendering order
    let mut sorted_nodes: Vec<&Node> = graph.nodes.values().collect();
    sorted_nodes.sort_by(|a, b| a.id.cmp(&b.id));

    for node in &sorted_nodes {
        max_x = max_x.max(node.x + node.width);
        max_y = max_y.max(node.y + node.height);
    }
    for sg in &graph.subgraphs {
        max_x = max_x.max(sg.x + sg.width);
        max_y = max_y.max(sg.y + sg.height);
    }

    // Add padding
    let mut grid = Grid::new(max_x + 2, max_y + 2);

    // 1. Render subgraphs first (background) and protect their borders
    for sg in &graph.subgraphs {
        draw_subgraph(&mut grid, sg, chars);
        protect_subgraph_borders(&mut grid, sg);
    }

    // 2. Render nodes in deterministic order
    for node in &sorted_nodes {
        draw_node(&mut grid, node, chars);
    }

    // 3. Build pathfinding grid for A* edge routing
    let path_grid = build_path_grid(graph, grid.width, grid.height);

    // 4. Render edges, tracking dropped labels
    let mut dropped_labels: Vec<edges::DroppedLabel> = Vec::new();
    let mut next_marker: usize = 1;

    for edge in &graph.edges {
        if let (Some(from), Some(to)) = (graph.nodes.get(&edge.from), graph.nodes.get(&edge.to)) {
            draw_edge(
                &mut grid,
                &path_grid,
                from,
                to,
                edge,
                chars,
                graph.direction,
                options.ascii,
                &mut dropped_labels,
                &mut next_marker,
            );
        }
    }

    let output = grid.to_string();

    // Apply max_width constraint if set (only to grid lines, not legend)
    let output = if let Some(max_width) = options.max_width {
        output
            .lines()
            .map(|line| {
                let char_count = line.chars().count();
                if char_count > max_width {
                    let mut truncated: String =
                        line.chars().take(max_width.saturating_sub(1)).collect();
                    truncated.push('…');
                    truncated
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        output
    };

    // Append legend for dropped labels
    if !dropped_labels.is_empty() {
        let mut result = output;
        result.push_str("\nLabels:");
        for dl in &dropped_labels {
            result.push_str(&format!("\n  {} {}", dl.marker, dl.label));
            warnings.push(DiagramWarning::LabelDropped {
                marker: dl.marker.clone(),
                edge_from: dl.from.clone(),
                edge_to: dl.to.clone(),
                label: dl.label.clone(),
            });
        }
        result
    } else {
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::compute_layout;
    use crate::parser::parse_mermaid;

    #[test]
    fn test_render_lr() {
        let mut graph = parse_mermaid("flowchart LR\nA[Start] --> B[End]").unwrap();
        compute_layout(&mut graph);
        let mut warnings = Vec::new();
        let output = render_graph(&graph, &RenderOptions::default(), &mut warnings);
        assert!(output.contains("Start"));
        assert!(output.contains("End"));
        assert!(output.contains("▶"));
    }

    #[test]
    fn test_render_tb() {
        let mut graph = parse_mermaid("flowchart TB\nA[Start] --> B[End]").unwrap();
        compute_layout(&mut graph);
        let mut warnings = Vec::new();
        let output = render_graph(&graph, &RenderOptions::default(), &mut warnings);
        assert!(output.contains("Start"));
        assert!(output.contains("End"));
        assert!(output.contains("▼"));
    }

    #[test]
    fn test_render_ascii() {
        let mut graph = parse_mermaid("flowchart LR\nA --> B").unwrap();
        compute_layout(&mut graph);
        let mut warnings = Vec::new();
        let output = render_graph(
            &graph,
            &RenderOptions {
                ascii: true,
                ..Default::default()
            },
            &mut warnings,
        );
        assert!(output.contains("+---+"));
        assert!(output.contains(">"));
        assert!(!output.contains("┌"));
    }

    #[test]
    fn test_render_rl() {
        let mut graph = parse_mermaid("flowchart RL\nA --> B").unwrap();
        compute_layout(&mut graph);
        let mut warnings = Vec::new();
        let output = render_graph(&graph, &RenderOptions::default(), &mut warnings);
        assert!(output.contains("◀"));
    }

    #[test]
    fn test_render_bt() {
        let mut graph = parse_mermaid("flowchart BT\nA --> B").unwrap();
        compute_layout(&mut graph);
        let mut warnings = Vec::new();
        let output = render_graph(&graph, &RenderOptions::default(), &mut warnings);
        assert!(output.contains("▲"));
    }

    #[test]
    fn test_render_rounded() {
        let mut graph = parse_mermaid("flowchart LR\nA(Rounded)").unwrap();
        compute_layout(&mut graph);
        let mut warnings = Vec::new();
        let output = render_graph(&graph, &RenderOptions::default(), &mut warnings);
        assert!(output.contains("Rounded"));
        assert!(output.contains("╭")); // Rounded corner
    }

    #[test]
    fn test_render_circle() {
        let mut graph = parse_mermaid("flowchart LR\nA((Circle))").unwrap();
        compute_layout(&mut graph);
        let mut warnings = Vec::new();
        let output = render_graph(&graph, &RenderOptions::default(), &mut warnings);
        assert!(output.contains("Circle"));
        assert!(output.contains("(")); // Circle sides
    }

    #[test]
    fn test_render_diamond() {
        let mut graph = parse_mermaid("flowchart LR\nA{Decision}").unwrap();
        compute_layout(&mut graph);
        let mut warnings = Vec::new();
        let output = render_graph(&graph, &RenderOptions::default(), &mut warnings);
        assert!(output.contains("Decision"));
        assert!(output.contains("<")); // Diamond sides
    }

    #[test]
    fn test_render_cylinder() {
        let mut graph = parse_mermaid("flowchart LR\nDB[(Database)]").unwrap();
        compute_layout(&mut graph);
        let mut warnings = Vec::new();
        let output = render_graph(&graph, &RenderOptions::default(), &mut warnings);
        assert!(output.contains("Database"));
    }

    #[test]
    fn test_render_max_width() {
        let mut graph = parse_mermaid("flowchart LR\nA[Start] --> B[End]").unwrap();
        compute_layout(&mut graph);
        let mut warnings = Vec::new();
        let output = render_graph(
            &graph,
            &RenderOptions {
                max_width: Some(15),
                ..Default::default()
            },
            &mut warnings,
        );
        // All lines should be truncated to max_width
        for line in output.lines() {
            assert!(
                line.chars().count() <= 15,
                "Line exceeds max_width: {} chars",
                line.chars().count()
            );
        }
        // Should contain ellipsis on truncated lines
        assert!(output.contains('…'));
    }

    #[test]
    fn test_render_max_width_no_truncation() {
        let mut graph = parse_mermaid("flowchart LR\nA --> B").unwrap();
        compute_layout(&mut graph);
        let mut warnings = Vec::new();
        let output = render_graph(
            &graph,
            &RenderOptions {
                max_width: Some(100), // Wide enough to not truncate
                ..Default::default()
            },
            &mut warnings,
        );
        // Should not contain ellipsis when no truncation needed
        assert!(!output.contains('…'));
    }

    #[test]
    fn test_diagonal_arrow_chars_exist() {
        use super::charset::{ASCII_CHARS, UNICODE_CHARS};
        // Verify diagonal arrow characters are defined
        assert_eq!(UNICODE_CHARS.arr_dr, '◢');
        assert_eq!(UNICODE_CHARS.arr_dl, '◣');
        assert_eq!(UNICODE_CHARS.arr_ur, '◥');
        assert_eq!(UNICODE_CHARS.arr_ul, '◤');

        // ASCII fallbacks
        assert_eq!(ASCII_CHARS.arr_dr, '\\');
        assert_eq!(ASCII_CHARS.arr_dl, '/');
        assert_eq!(ASCII_CHARS.arr_ur, '/');
        assert_eq!(ASCII_CHARS.arr_ul, '\\');
    }

    #[test]
    fn test_get_arrow_for_direction() {
        use super::charset::UNICODE_CHARS;
        use super::edges::get_arrow_for_direction;
        use crate::pathfinding::Pos;

        // Test cardinal directions
        assert_eq!(
            get_arrow_for_direction(Pos::new(0, 0), Pos::new(1, 0), '?', &UNICODE_CHARS),
            '▶'
        );
        assert_eq!(
            get_arrow_for_direction(Pos::new(1, 0), Pos::new(0, 0), '?', &UNICODE_CHARS),
            '◀'
        );
        assert_eq!(
            get_arrow_for_direction(Pos::new(0, 0), Pos::new(0, 1), '?', &UNICODE_CHARS),
            '▼'
        );
        assert_eq!(
            get_arrow_for_direction(Pos::new(0, 1), Pos::new(0, 0), '?', &UNICODE_CHARS),
            '▲'
        );

        // Test diagonal directions
        assert_eq!(
            get_arrow_for_direction(Pos::new(0, 0), Pos::new(1, 1), '?', &UNICODE_CHARS),
            '◢'
        );
        assert_eq!(
            get_arrow_for_direction(Pos::new(1, 0), Pos::new(0, 1), '?', &UNICODE_CHARS),
            '◣'
        );
        assert_eq!(
            get_arrow_for_direction(Pos::new(0, 1), Pos::new(1, 0), '?', &UNICODE_CHARS),
            '◥'
        );
        assert_eq!(
            get_arrow_for_direction(Pos::new(1, 1), Pos::new(0, 0), '?', &UNICODE_CHARS),
            '◤'
        );
    }
}
