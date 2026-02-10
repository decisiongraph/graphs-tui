use crate::types::{Direction, Graph, NodeId, RenderOptions};
use std::collections::{HashMap, VecDeque};

const MIN_NODE_WIDTH: usize = 5;
const NODE_HEIGHT: usize = 3;
const DEFAULT_HORIZONTAL_GAP: usize = 8;
const DEFAULT_VERTICAL_GAP: usize = 4;
const MIN_GAP: usize = 2;

const SUBGRAPH_PADDING: usize = 2;

/// Compute layout for all nodes in the graph
///
/// Returns a list of warnings (e.g., cycle detected).
pub fn compute_layout(graph: &mut Graph) -> Vec<String> {
    compute_layout_with_options(graph, &RenderOptions::default())
}

/// Compute layout for all nodes with render options (considers max_width)
///
/// Returns a list of warnings (e.g., cycle detected).
pub fn compute_layout_with_options(graph: &mut Graph, options: &RenderOptions) -> Vec<String> {
    let mut warnings = Vec::new();

    // 1. Compute node sizes (use chars().count() for proper Unicode handling)
    for node in graph.nodes.values_mut() {
        node.width = (node.label.chars().count() + 2).max(MIN_NODE_WIDTH);
        node.height = NODE_HEIGHT;
    }

    // 2. Topological layering
    let layers = assign_layers(graph, &mut warnings);

    // 3. Calculate gaps based on available width
    let (h_gap, v_gap) = calculate_gaps(graph, &layers, options.max_width);

    // 4. Position assignment based on direction with calculated gaps
    assign_coordinates_with_gaps(graph, &layers, h_gap, v_gap);

    // 5. Compute subgraph bounding boxes
    compute_subgraph_bounds(graph);

    warnings
}

/// Calculate adaptive gaps based on available width
fn calculate_gaps(
    graph: &Graph,
    layers: &HashMap<NodeId, usize>,
    max_width: Option<usize>,
) -> (usize, usize) {
    let max_width = match max_width {
        Some(w) => w,
        None => return (DEFAULT_HORIZONTAL_GAP, DEFAULT_VERTICAL_GAP),
    };

    // Group nodes by layer
    let mut layers_map: HashMap<usize, Vec<&NodeId>> = HashMap::new();
    let mut max_layer = 0;

    for (id, &layer) in layers {
        layers_map.entry(layer).or_default().push(id);
        max_layer = max_layer.max(layer);
    }

    // Calculate natural width with default gaps (for horizontal layouts)
    if graph.direction.is_horizontal() {
        let mut total_width = 0;
        for l in 0..=max_layer {
            let nodes_in_layer = layers_map.get(&l).map(|v| v.as_slice()).unwrap_or(&[]);
            let layer_max_width = nodes_in_layer
                .iter()
                .filter_map(|id| graph.nodes.get(*id))
                .map(|n| n.width)
                .max()
                .unwrap_or(0);
            total_width += layer_max_width;
        }
        total_width += max_layer * DEFAULT_HORIZONTAL_GAP;

        // If natural width exceeds max_width, reduce horizontal gap
        if total_width > max_width && max_layer > 0 {
            let node_width = total_width - max_layer * DEFAULT_HORIZONTAL_GAP;
            let available_for_gaps = max_width.saturating_sub(node_width);
            let new_gap = (available_for_gaps / max_layer).max(MIN_GAP);
            return (new_gap, DEFAULT_VERTICAL_GAP);
        }
    }

    (DEFAULT_HORIZONTAL_GAP, DEFAULT_VERTICAL_GAP)
}

/// Compute bounding boxes for all subgraphs
fn compute_subgraph_bounds(graph: &mut Graph) {
    for sg in &mut graph.subgraphs {
        if sg.nodes.is_empty() {
            continue;
        }

        let mut min_x = usize::MAX;
        let mut min_y = usize::MAX;
        let mut max_x = 0;
        let mut max_y = 0;

        for node_id in &sg.nodes {
            if let Some(node) = graph.nodes.get(node_id) {
                min_x = min_x.min(node.x);
                min_y = min_y.min(node.y);
                max_x = max_x.max(node.x + node.width);
                max_y = max_y.max(node.y + node.height);
            }
        }

        if min_x != usize::MAX {
            // Add padding around the subgraph
            sg.x = min_x.saturating_sub(SUBGRAPH_PADDING);
            sg.y = min_y.saturating_sub(SUBGRAPH_PADDING + 1); // Extra space for label
            sg.width = (max_x - min_x) + SUBGRAPH_PADDING * 2;
            sg.height = (max_y - min_y) + SUBGRAPH_PADDING * 2 + 1;
        }
    }
}

/// Assign layer numbers using Kahn's algorithm
fn assign_layers(graph: &Graph, warnings: &mut Vec<String>) -> HashMap<NodeId, usize> {
    let mut node_layers: HashMap<NodeId, usize> = HashMap::new();
    let mut in_degree: HashMap<NodeId, usize> = HashMap::new();

    // Initialize
    for id in graph.nodes.keys() {
        in_degree.insert(id.clone(), 0);
        node_layers.insert(id.clone(), 0);
    }

    // Count in-degrees
    for edge in &graph.edges {
        *in_degree.entry(edge.to.clone()).or_insert(0) += 1;
    }

    // Start with nodes that have no incoming edges
    let mut queue: VecDeque<NodeId> = VecDeque::new();
    for (id, &degree) in &in_degree {
        if degree == 0 {
            queue.push_back(id.clone());
        }
    }

    let mut processed = 0;
    while let Some(u) = queue.pop_front() {
        processed += 1;

        // Find all neighbors (nodes that u points to)
        let neighbors: Vec<NodeId> = graph
            .edges
            .iter()
            .filter(|e| e.from == u)
            .map(|e| e.to.clone())
            .collect();

        for v in neighbors {
            // Update layer to be at least one more than predecessor
            let u_layer = *node_layers.get(&u).unwrap_or(&0);
            let v_layer = node_layers.entry(v.clone()).or_insert(0);
            *v_layer = (*v_layer).max(u_layer + 1);

            // Decrement in-degree
            if let Some(deg) = in_degree.get_mut(&v) {
                *deg -= 1;
                if *deg == 0 {
                    queue.push_back(v);
                }
            }
        }
    }

    // Check for cycles
    if processed < graph.nodes.len() {
        warnings.push("Cycle detected in graph. Layout may be imperfect.".to_string());
    }

    node_layers
}

/// Assign x,y coordinates based on layers and direction with configurable gaps
fn assign_coordinates_with_gaps(
    graph: &mut Graph,
    node_layers: &HashMap<NodeId, usize>,
    h_gap: usize,
    v_gap: usize,
) {
    let direction = graph.direction;

    // Group nodes by layer
    let mut layers_map: HashMap<usize, Vec<NodeId>> = HashMap::new();
    let mut max_layer = 0;

    for (id, &layer) in node_layers {
        layers_map.entry(layer).or_default().push(id.clone());
        max_layer = max_layer.max(layer);
    }

    // Calculate layer dimensions
    let mut layer_widths: HashMap<usize, usize> = HashMap::new();
    let mut layer_heights: HashMap<usize, usize> = HashMap::new();

    for l in 0..=max_layer {
        let nodes_in_layer = layers_map.get(&l).map(|v| v.as_slice()).unwrap_or(&[]);
        let mut max_w = 0;
        let mut max_h = 0;
        let mut total_w = 0;
        let mut total_h = 0;

        for id in nodes_in_layer {
            if let Some(node) = graph.nodes.get(id) {
                max_w = max_w.max(node.width);
                max_h = max_h.max(node.height);
                total_w += node.width + h_gap;
                total_h += node.height + v_gap;
            }
        }

        if direction.is_horizontal() {
            layer_widths.insert(l, max_w);
            layer_heights.insert(l, total_h.saturating_sub(v_gap));
        } else {
            layer_widths.insert(l, total_w.saturating_sub(h_gap));
            layer_heights.insert(l, max_h);
        }
    }

    let max_total_width = layer_widths.values().copied().max().unwrap_or(0);
    let max_total_height = layer_heights.values().copied().max().unwrap_or(0);

    if direction.is_horizontal() {
        let mut current_x = 0;
        for l in 0..=max_layer {
            let layer_idx = match direction {
                Direction::LR => l,
                Direction::RL => max_layer - l,
                _ => l,
            };

            let nodes_in_layer = layers_map.get(&layer_idx).cloned().unwrap_or_default();
            let layer_h = *layer_heights.get(&layer_idx).unwrap_or(&0);
            let mut start_y = (max_total_height.saturating_sub(layer_h)) / 2;

            for id in nodes_in_layer {
                if let Some(node) = graph.nodes.get_mut(&id) {
                    node.x = current_x;
                    node.y = start_y;
                    start_y += node.height + v_gap;
                }
            }

            current_x += layer_widths.get(&layer_idx).unwrap_or(&0) + h_gap;
        }
    } else {
        let mut current_y = 0;
        for l in 0..=max_layer {
            let layer_idx = match direction {
                Direction::TB => l,
                Direction::BT => max_layer - l,
                _ => l,
            };

            let nodes_in_layer = layers_map.get(&layer_idx).cloned().unwrap_or_default();
            let layer_w = *layer_widths.get(&layer_idx).unwrap_or(&0);
            let mut start_x = (max_total_width.saturating_sub(layer_w)) / 2;

            for id in nodes_in_layer {
                if let Some(node) = graph.nodes.get_mut(&id) {
                    node.x = start_x;
                    node.y = current_y;
                    start_x += node.width + h_gap;
                }
            }

            current_y += layer_heights.get(&layer_idx).unwrap_or(&0) + v_gap;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_mermaid;

    #[test]
    fn test_layout_lr() {
        let mut graph = parse_mermaid("flowchart LR\nA --> B").unwrap();
        let warnings = compute_layout(&mut graph);

        let a = graph.nodes.get("A").unwrap();
        let b = graph.nodes.get("B").unwrap();

        assert!(a.x < b.x);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_layout_tb() {
        let mut graph = parse_mermaid("flowchart TB\nA --> B").unwrap();
        let warnings = compute_layout(&mut graph);

        let a = graph.nodes.get("A").unwrap();
        let b = graph.nodes.get("B").unwrap();

        assert!(a.y < b.y);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_node_sizes() {
        let mut graph = parse_mermaid("flowchart LR\nA[Hello World]").unwrap();
        compute_layout(&mut graph);

        let a = graph.nodes.get("A").unwrap();
        assert_eq!(a.width, "Hello World".len() + 2);
        assert_eq!(a.height, NODE_HEIGHT);
    }

    #[test]
    fn test_cycle_produces_warning() {
        let mut graph = parse_mermaid("flowchart LR\nA --> B\nB --> C\nC --> A").unwrap();
        let warnings = compute_layout(&mut graph);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("Cycle"));
    }

    #[test]
    fn test_acyclic_no_warning() {
        let mut graph = parse_mermaid("flowchart LR\nA --> B\nB --> C\nA --> C").unwrap();
        let warnings = compute_layout(&mut graph);
        assert!(warnings.is_empty());
    }
}
