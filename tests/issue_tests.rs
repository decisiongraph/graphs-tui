//! Tests for GitHub issues
use graphs_tui::{render_d2_to_tui, render_mermaid_to_tui, RenderOptions};

/// Issue #7: Warnings returned in RenderResult instead of eprintln
#[test]
fn test_issue_7_cycle_warning_in_result() {
    // Cyclical graph: A -> B -> C -> A
    let input = r#"flowchart LR
A --> B
B --> C
C --> A"#;
    let result = render_mermaid_to_tui(input, RenderOptions::default()).unwrap();
    assert!(
        !result.warnings.is_empty(),
        "Cycle should produce a warning"
    );
    assert!(
        result.warnings[0].contains("Cycle"),
        "Warning should mention cycle"
    );
    // Output should still render
    assert!(result.output.contains("A"));
    assert!(result.output.contains("B"));
    assert!(result.output.contains("C"));
}

/// Issue #7: Non-cyclic graphs produce no warnings
#[test]
fn test_issue_7_no_warning_without_cycle() {
    let input = "flowchart LR\nA --> B --> C";
    let result = render_mermaid_to_tui(input, RenderOptions::default()).unwrap();
    assert!(result.warnings.is_empty(), "No cycle means no warnings");
}

/// Issue #7: D2 cyclic graph also produces warning
#[test]
fn test_issue_7_d2_cycle_warning() {
    let input = r#"
A -> B
B -> C
C -> A
"#;
    let result = render_d2_to_tui(input, RenderOptions::default()).unwrap();
    assert!(
        !result.warnings.is_empty(),
        "D2 cycle should produce a warning"
    );
}

/// Issue #7: Pie chart has no warnings
#[test]
fn test_issue_7_pie_no_warnings() {
    let input = r#"pie
    "A" : 50
    "B" : 50
"#;
    let result = graphs_tui::render_pie_chart(input, RenderOptions::default()).unwrap();
    assert!(result.warnings.is_empty());
}

// ── Issue #8: Rendering artifacts ──────────────────────────────────────

/// Issue #8: Cylinder renders 5 rows with ├───┤ separators
#[test]
fn test_issue_8_cylinder_5_rows() {
    let input = "flowchart LR\nDB[(Database)]";
    let result = render_mermaid_to_tui(input, RenderOptions::default()).unwrap();
    let output = &result.output;
    assert!(output.contains('├'), "Cylinder should have ├ T-junction");
    assert!(output.contains('┤'), "Cylinder should have ┤ T-junction");
    assert!(output.contains('╭'), "Cylinder should have ╭ top-left");
    assert!(output.contains('╰'), "Cylinder should have ╰ bottom-left");
    assert!(output.contains("Database"), "Label should be present");

    // Verify 5-row structure: find the cylinder lines
    let lines: Vec<&str> = output.lines().collect();
    // Find the line with ╭ (top of cylinder)
    let top = lines.iter().position(|l| l.contains('╭')).unwrap();
    let bot = lines.iter().position(|l| l.contains('╰')).unwrap();
    assert_eq!(bot - top, 4, "Cylinder should span exactly 5 rows (0..4)");
    // Row 1 and 3 should have ├ separators
    assert!(lines[top + 1].contains('├'), "Row 1 should have ├");
    assert!(lines[top + 3].contains('├'), "Row 3 should have ├");
}

/// Issue #8: Edge connects to cylinder midpoint (height/2), not y+1
#[test]
fn test_issue_8_edge_connects_cylinder_midpoint() {
    let input = "flowchart LR\nA[Start] --> DB[(Database)]";
    let result = render_mermaid_to_tui(input, RenderOptions::default()).unwrap();
    let output = &result.output;

    // The arrow should be on the same row as the label (midpoint)
    let lines: Vec<&str> = output.lines().collect();
    let arrow_line = lines
        .iter()
        .position(|l| l.contains('▶'))
        .expect("Should have arrow");
    let label_line = lines
        .iter()
        .position(|l| l.contains("Database"))
        .expect("Should have Database label");
    assert_eq!(
        arrow_line, label_line,
        "Arrow should connect at cylinder midpoint (same row as label)"
    );
}

/// Issue #8: Subgraph ║ borders not corrupted by nodes
#[test]
fn test_issue_8_subgraph_borders_preserved() {
    let input = r#"flowchart LR
subgraph sg1 [Group]
    A[Node]
end"#;
    let result = render_mermaid_to_tui(input, RenderOptions::default()).unwrap();
    let output = &result.output;
    // Subgraph should have ║ vertical borders
    assert!(
        output.contains('║'),
        "Subgraph should have ║ vertical borders"
    );
    assert!(output.contains('╔'), "Subgraph should have ╔ corner");
    assert!(output.contains('╗'), "Subgraph should have ╗ corner");
    assert!(output.contains('╚'), "Subgraph should have ╚ corner");
    assert!(output.contains('╝'), "Subgraph should have ╝ corner");
}

/// Issue #8: Subgraph labels not corrupted by edges
#[test]
fn test_issue_8_subgraph_labels_intact() {
    let input = r#"flowchart TB
subgraph sg1 [My Group]
    A[Node1]
    B[Node2]
end
A --> B"#;
    let result = render_mermaid_to_tui(input, RenderOptions::default()).unwrap();
    let output = &result.output;
    assert!(
        output.contains("My Group"),
        "Subgraph label should be intact"
    );
}
