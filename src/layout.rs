use crate::types::{Direction, Graph, NodeId};
use std::collections::{HashMap, VecDeque};

const MIN_NODE_WIDTH: usize = 5;
const NODE_HEIGHT: usize = 3;
const HORIZONTAL_GAP: usize = 8;
const VERTICAL_GAP: usize = 4;

const SUBGRAPH_PADDING: usize = 2;

/// Compute layout for all nodes in the graph
pub fn compute_layout(graph: &mut Graph) {
    // 1. Compute node sizes
    for node in graph.nodes.values_mut() {
        node.width = (node.label.len() + 2).max(MIN_NODE_WIDTH);
        node.height = NODE_HEIGHT;
    }

    // 2. Topological layering
    let layers = assign_layers(graph);

    // 3. Position assignment based on direction
    assign_coordinates(graph, &layers);

    // 4. Compute subgraph bounding boxes
    compute_subgraph_bounds(graph);
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
fn assign_layers(graph: &Graph) -> HashMap<NodeId, usize> {
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
        eprintln!("Warning: Cycle detected in graph. Layout may be imperfect.");
    }

    node_layers
}

/// Assign x,y coordinates based on layers and direction
fn assign_coordinates(graph: &mut Graph, node_layers: &HashMap<NodeId, usize>) {
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
                total_w += node.width + HORIZONTAL_GAP;
                total_h += node.height + VERTICAL_GAP;
            }
        }

        if direction.is_horizontal() {
            layer_widths.insert(l, max_w);
            layer_heights.insert(l, total_h.saturating_sub(VERTICAL_GAP));
        } else {
            layer_widths.insert(l, total_w.saturating_sub(HORIZONTAL_GAP));
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
                    start_y += node.height + VERTICAL_GAP;
                }
            }

            current_x += layer_widths.get(&layer_idx).unwrap_or(&0) + HORIZONTAL_GAP;
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
                    start_x += node.width + HORIZONTAL_GAP;
                }
            }

            current_y += layer_heights.get(&layer_idx).unwrap_or(&0) + VERTICAL_GAP;
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
        compute_layout(&mut graph);

        let a = graph.nodes.get("A").unwrap();
        let b = graph.nodes.get("B").unwrap();

        // A should be to the left of B
        assert!(a.x < b.x);
    }

    #[test]
    fn test_layout_tb() {
        let mut graph = parse_mermaid("flowchart TB\nA --> B").unwrap();
        compute_layout(&mut graph);

        let a = graph.nodes.get("A").unwrap();
        let b = graph.nodes.get("B").unwrap();

        // A should be above B
        assert!(a.y < b.y);
    }

    #[test]
    fn test_node_sizes() {
        let mut graph = parse_mermaid("flowchart LR\nA[Hello World]").unwrap();
        compute_layout(&mut graph);

        let a = graph.nodes.get("A").unwrap();
        assert_eq!(a.width, "Hello World".len() + 2);
        assert_eq!(a.height, NODE_HEIGHT);
    }
}
