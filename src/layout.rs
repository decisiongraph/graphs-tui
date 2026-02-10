use crate::types::{DiagramWarning, Direction, Graph, NodeId, NodeShape, RenderOptions, TableField};
use std::collections::{HashMap, HashSet, VecDeque};

const MIN_NODE_WIDTH: usize = 5;
const NODE_HEIGHT: usize = 3;
const MIN_GAP: usize = 2;

const SUBGRAPH_PADDING: usize = 2;

/// Compute layout for all nodes in the graph
///
/// Returns a list of warnings (e.g., cycle detected).
pub fn compute_layout(graph: &mut Graph) -> Vec<DiagramWarning> {
    compute_layout_with_options(graph, &RenderOptions::default())
}

/// Compute layout for all nodes with render options (considers max_width)
///
/// Returns a list of warnings (e.g., cycle detected).
pub fn compute_layout_with_options(
    graph: &mut Graph,
    options: &RenderOptions,
) -> Vec<DiagramWarning> {
    let mut warnings = Vec::new();

    // Border padding affects node width (text + 2*border_padding)
    let text_padding = options.border_padding * 2;

    // 1. Compute node sizes (use chars().count() for proper Unicode handling)
    for node in graph.nodes.values_mut() {
        node.width = (node.label.chars().count() + text_padding).max(MIN_NODE_WIDTH);
        node.height = NODE_HEIGHT;
        if node.shape == NodeShape::Cylinder {
            node.height = 5;
        }
        // sql_table/class with fields: header + separator + fields + border
        if node.shape == NodeShape::Table && !node.fields.is_empty() {
            // Width: max of label and all field lines
            for field in &node.fields {
                let field_len = format_field_width(field);
                node.width = node.width.max(field_len + 2 + text_padding); // 2 for borders + padding
            }
            // Height: top border + label row + separator + field rows + bottom border
            node.height = 3 + node.fields.len(); // 3 = top + label + separator, then 1 per field, +1 bottom handled by renderer
        }
    }

    // 2. Topological layering
    let layers = assign_layers(graph, &mut warnings);

    // 3. Calculate gaps based on available width and user-specified padding
    let (h_gap, v_gap) = calculate_gaps(graph, &layers, options);

    // 4. Position assignment based on direction with calculated gaps
    assign_coordinates_with_gaps(graph, &layers, h_gap, v_gap);

    // 5. Compute subgraph bounding boxes
    compute_subgraph_bounds(graph);

    warnings
}

/// Calculate adaptive gaps based on available width and user options
fn calculate_gaps(
    graph: &Graph,
    layers: &HashMap<NodeId, usize>,
    options: &RenderOptions,
) -> (usize, usize) {
    let h_gap = options.padding_x;
    let v_gap = options.padding_y;

    let max_width = match options.max_width {
        Some(w) => w,
        None => return (h_gap, v_gap),
    };

    // Group nodes by layer (sorted for determinism)
    let mut layers_map: HashMap<usize, Vec<&NodeId>> = HashMap::new();
    let mut max_layer = 0;

    for (id, &layer) in layers {
        layers_map.entry(layer).or_default().push(id);
        max_layer = max_layer.max(layer);
    }
    for nodes in layers_map.values_mut() {
        nodes.sort();
    }

    // Calculate natural width with user-specified gaps (for horizontal layouts)
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
        total_width += max_layer * h_gap;

        // If natural width exceeds max_width, reduce horizontal gap
        if total_width > max_width && max_layer > 0 {
            let node_width = total_width - max_layer * h_gap;
            let available_for_gaps = max_width.saturating_sub(node_width);
            let new_gap = (available_for_gaps / max_layer).max(MIN_GAP);
            return (new_gap, v_gap);
        }
    }

    (h_gap, v_gap)
}

/// Calculate display width of a table field
fn format_field_width(field: &TableField) -> usize {
    let mut len = field.name.chars().count();
    if let Some(ref ti) = field.type_info {
        len += 2 + ti.chars().count(); // ": type"
    }
    if let Some(ref c) = field.constraint {
        len += 1 + constraint_abbrev(c).len(); // " [PK]"
    }
    len
}

/// Abbreviate common constraints
fn constraint_abbrev(constraint: &str) -> String {
    match constraint {
        "primary_key" => "[PK]".to_string(),
        "foreign_key" => "[FK]".to_string(),
        "unique" => "[UQ]".to_string(),
        "not_null" => "[NN]".to_string(),
        other => format!("[{}]", other),
    }
}

/// Compute bounding boxes for all subgraphs.
/// Process leaf subgraphs first (those with no children), then parents,
/// so parent bounds include child subgraph bounds.
fn compute_subgraph_bounds(graph: &mut Graph) {
    // Build child→parent relationships
    let sg_count = graph.subgraphs.len();
    let sg_ids: Vec<String> = graph.subgraphs.iter().map(|sg| sg.id.clone()).collect();
    let sg_parents: Vec<Option<String>> = graph.subgraphs.iter().map(|sg| sg.parent.clone()).collect();

    // Determine processing order: leaf subgraphs first (no children)
    let mut has_children: std::collections::HashSet<String> = std::collections::HashSet::new();
    for parent in &sg_parents {
        if let Some(ref p) = parent {
            has_children.insert(p.clone());
        }
    }

    // Simple two-pass: first process subgraphs without children, then those with children
    let mut processed: std::collections::HashSet<String> = std::collections::HashSet::new();

    // Multiple passes until all processed (handles deep nesting)
    for _ in 0..sg_count + 1 {
        for i in 0..sg_count {
            let sg_id = &sg_ids[i];
            if processed.contains(sg_id) {
                continue;
            }

            // Check if all children are processed
            let all_children_done = sg_ids.iter().enumerate().all(|(j, child_id)| {
                if sg_parents[j].as_ref() == Some(sg_id) {
                    processed.contains(child_id)
                } else {
                    true
                }
            });

            if !all_children_done {
                continue;
            }

            // Compute bounds from member nodes
            let sg = &graph.subgraphs[i];
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

            // Include child subgraph bounds
            for (j, _child_id) in sg_ids.iter().enumerate() {
                if sg_parents[j].as_ref() == Some(sg_id) {
                    let child = &graph.subgraphs[j];
                    if child.width > 0 && child.height > 0 {
                        min_x = min_x.min(child.x);
                        min_y = min_y.min(child.y);
                        max_x = max_x.max(child.x + child.width);
                        max_y = max_y.max(child.y + child.height);
                    }
                }
            }

            if min_x != usize::MAX {
                graph.subgraphs[i].x = min_x.saturating_sub(SUBGRAPH_PADDING);
                graph.subgraphs[i].y = min_y.saturating_sub(SUBGRAPH_PADDING + 1);
                graph.subgraphs[i].width = (max_x - min_x) + SUBGRAPH_PADDING * 2;
                graph.subgraphs[i].height = (max_y - min_y) + SUBGRAPH_PADDING * 2 + 1;
            }

            processed.insert(sg_id.clone());
        }

        if processed.len() == sg_count {
            break;
        }
    }
}

/// Assign layer numbers using Kahn's algorithm with cycle-breaking.
///
/// Standard Kahn's processes nodes with in_degree=0. When the queue empties
/// but unprocessed nodes remain, a cycle exists. We force-process the stuck
/// node that appears earliest as a "from" in the edge list (preserving the
/// user's intended flow direction), then continue Kahn's.
fn assign_layers(graph: &Graph, warnings: &mut Vec<DiagramWarning>) -> HashMap<NodeId, usize> {
    let mut node_layers: HashMap<NodeId, usize> = HashMap::new();
    let mut in_degree: HashMap<NodeId, usize> = HashMap::new();
    let mut processed: HashSet<NodeId> = HashSet::new();

    // Initialize
    for id in graph.nodes.keys() {
        in_degree.insert(id.clone(), 0);
        node_layers.insert(id.clone(), 0);
    }

    // Count in-degrees
    for edge in &graph.edges {
        *in_degree.entry(edge.to.clone()).or_insert(0) += 1;
    }

    // Build first-appearance-as-from index for deterministic cycle breaking.
    // Nodes that appear earlier as edge sources are treated as more "source-like"
    // when breaking cycles.
    let mut first_from_idx: HashMap<&str, usize> = HashMap::new();
    for (i, edge) in graph.edges.iter().enumerate() {
        first_from_idx.entry(edge.from.as_str()).or_insert(i);
    }

    // Start with nodes that have no incoming edges (sorted for determinism)
    let mut queue: VecDeque<NodeId> = VecDeque::new();
    let mut zero_in: Vec<&NodeId> = in_degree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(id, _)| id)
        .collect();
    zero_in.sort();
    for id in zero_in {
        queue.push_back(id.clone());
    }

    let total = graph.nodes.len();
    let mut all_cycle_nodes: HashSet<String> = HashSet::new();

    loop {
        // Standard Kahn's processing
        while let Some(u) = queue.pop_front() {
            if processed.contains(&u) {
                continue;
            }
            processed.insert(u.clone());

            // Find neighbors, skipping already-processed nodes
            let mut neighbors: Vec<NodeId> = graph
                .edges
                .iter()
                .filter(|e| e.from == u && !processed.contains(&e.to))
                .map(|e| e.to.clone())
                .collect();
            neighbors.sort();
            neighbors.dedup();

            for v in &neighbors {
                let u_layer = *node_layers.get(&u).unwrap_or(&0);
                let v_layer = node_layers.entry(v.clone()).or_insert(0);
                *v_layer = (*v_layer).max(u_layer + 1);

                if let Some(deg) = in_degree.get_mut(v) {
                    *deg = deg.saturating_sub(1);
                    if *deg == 0 {
                        queue.push_back(v.clone());
                    }
                }
            }
        }

        if processed.len() >= total {
            break;
        }

        // Cycle detected — collect stuck nodes
        let mut stuck: Vec<NodeId> = in_degree
            .iter()
            .filter(|(id, _)| !processed.contains(*id))
            .map(|(id, _)| id.clone())
            .collect();

        // Record only nodes that have outgoing edges to other stuck nodes
        // (actual cycle participants, not just downstream nodes)
        let stuck_set: HashSet<&str> = stuck.iter().map(|s| s.as_str()).collect();
        for n in &stuck {
            let has_outgoing_to_stuck = graph
                .edges
                .iter()
                .any(|e| e.from == *n && stuck_set.contains(e.to.as_str()));
            if has_outgoing_to_stuck {
                all_cycle_nodes.insert(n.clone());
            }
        }

        // Force-process the stuck node that appears earliest as an edge source
        stuck.sort_by(|a, b| {
            let fa = first_from_idx.get(a.as_str()).copied().unwrap_or(usize::MAX);
            let fb = first_from_idx.get(b.as_str()).copied().unwrap_or(usize::MAX);
            fa.cmp(&fb).then(a.cmp(b))
        });

        if let Some(forced) = stuck.first() {
            in_degree.insert(forced.clone(), 0);
            queue.push_back(forced.clone());
        }
    }

    if !all_cycle_nodes.is_empty() {
        let mut cycle_nodes: Vec<String> = all_cycle_nodes.into_iter().collect();
        cycle_nodes.sort();
        warnings.push(DiagramWarning::CycleDetected { nodes: cycle_nodes });
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

    // Group nodes by layer, sort within each layer for determinism
    let mut layers_map: HashMap<usize, Vec<NodeId>> = HashMap::new();
    let mut max_layer = 0;

    for (id, &layer) in node_layers {
        layers_map.entry(layer).or_default().push(id.clone());
        max_layer = max_layer.max(layer);
    }
    for nodes in layers_map.values_mut() {
        nodes.sort();
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
        assert!(warnings[0].to_string().contains("Cycle"));
    }

    #[test]
    fn test_acyclic_no_warning() {
        let mut graph = parse_mermaid("flowchart LR\nA --> B\nB --> C\nA --> C").unwrap();
        let warnings = compute_layout(&mut graph);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_custom_padding() {
        let mut graph = parse_mermaid("flowchart LR\nA --> B").unwrap();
        let options = RenderOptions {
            padding_x: 20,
            padding_y: 10,
            ..Default::default()
        };
        compute_layout_with_options(&mut graph, &options);
        let a = graph.nodes.get("A").unwrap();
        let b = graph.nodes.get("B").unwrap();
        // With larger padding_x, B should be further to the right
        assert!(b.x - (a.x + a.width) >= 20);
    }

    #[test]
    fn test_border_padding_affects_width() {
        let mut graph1 = parse_mermaid("flowchart LR\nA[Test]").unwrap();
        let mut graph2 = parse_mermaid("flowchart LR\nA[Test]").unwrap();

        let opts1 = RenderOptions {
            border_padding: 1,
            ..Default::default()
        };
        let opts2 = RenderOptions {
            border_padding: 3,
            ..Default::default()
        };

        compute_layout_with_options(&mut graph1, &opts1);
        compute_layout_with_options(&mut graph2, &opts2);

        let w1 = graph1.nodes.get("A").unwrap().width;
        let w2 = graph2.nodes.get("A").unwrap().width;
        // Larger border_padding should result in wider nodes
        assert!(w2 > w1);
    }
}
