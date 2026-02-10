//! Tests for GitHub issues
use graphs_tui::{
    check, is_supported, render, render_d2_to_tui, render_mermaid_to_tui, DiagramWarning,
    RenderOptions, SUPPORTED_LANGUAGES,
};

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
        result.warnings[0].to_string().contains("Cycle"),
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

// ── Issue #9: Deterministic layout, dropped edge labels, cycle warning UX ──

/// Issue #9: Same input produces identical output across multiple runs (Mermaid)
#[test]
fn test_issue_9_deterministic_mermaid() {
    let input = r#"flowchart LR
A[Start] --> B[Middle]
A --> C[Other]
B --> D[End]
C --> D"#;
    let first = render_mermaid_to_tui(input, RenderOptions::default())
        .unwrap()
        .output;
    for i in 1..20 {
        let result = render_mermaid_to_tui(input, RenderOptions::default())
            .unwrap()
            .output;
        assert_eq!(first, result, "Mermaid run {i} differs from first run");
    }
}

/// Issue #9: Same input produces identical output across multiple runs (D2)
#[test]
fn test_issue_9_deterministic_d2() {
    let input = r#"
A -> B
A -> C
B -> D
C -> D
"#;
    let first = render_d2_to_tui(input, RenderOptions::default())
        .unwrap()
        .output;
    for i in 1..20 {
        let result = render_d2_to_tui(input, RenderOptions::default())
            .unwrap()
            .output;
        assert_eq!(first, result, "D2 run {i} differs from first run");
    }
}

/// Issue #9: D2 cycle graph determinism (original reproducer from #9)
#[test]
fn test_issue_9_deterministic_d2_with_cycle() {
    let input = r#"
users: Users
api: Production API
pgbouncer: PgBouncer { shape: cylinder }
analytics: Analytics Query
users -> api: requests
api -> pgbouncer: need conn
analytics -> pgbouncer: 60 conns held
api -> users: 503 errors
"#;
    let first = render_d2_to_tui(input, RenderOptions::default())
        .unwrap()
        .output;
    for i in 1..20 {
        let result = render_d2_to_tui(input, RenderOptions::default())
            .unwrap()
            .output;
        assert_eq!(first, result, "D2 cycle run {i} differs");
    }
}

/// Issue #9: Edge label dropped to legend when edge is too short
#[test]
fn test_issue_9_edge_label_legend() {
    // Use very short node names with a long label to force the label to not fit
    let input = "flowchart LR\nA -->|This is a very long label that will not fit| B";
    let result = render_mermaid_to_tui(input, RenderOptions::default()).unwrap();

    // Legend should appear
    assert!(
        result.output.contains("Labels:"),
        "Legend section should appear when label is dropped"
    );
    assert!(
        result
            .output
            .contains("This is a very long label that will not fit"),
        "Legend should contain the dropped label text"
    );

    // Should have a LabelDropped warning
    let has_label_warning = result
        .warnings
        .iter()
        .any(|w| matches!(w, DiagramWarning::LabelDropped { .. }));
    assert!(has_label_warning, "Should have a LabelDropped warning");
}

/// Issue #9: Cycle warning includes node names
#[test]
fn test_issue_9_cycle_warning_nodes() {
    let input = r#"flowchart LR
X --> Y
Y --> Z
Z --> X"#;
    let result = render_mermaid_to_tui(input, RenderOptions::default()).unwrap();
    assert_eq!(result.warnings.len(), 1);

    match &result.warnings[0] {
        DiagramWarning::CycleDetected { nodes } => {
            assert!(nodes.contains(&"X".to_string()), "Should contain X");
            assert!(nodes.contains(&"Y".to_string()), "Should contain Y");
            assert!(nodes.contains(&"Z".to_string()), "Should contain Z");
            // Nodes should be sorted
            assert_eq!(nodes, &["X", "Y", "Z"]);
        }
        other => panic!("Expected CycleDetected, got: {other:?}"),
    }
}

/// Issue #9: DiagramWarning Display impl
#[test]
fn test_issue_9_warning_display() {
    let w = DiagramWarning::CycleDetected {
        nodes: vec!["A".into(), "B".into()],
    };
    assert_eq!(w.to_string(), "Cycle detected involving nodes: A, B");

    let w2 = DiagramWarning::LabelDropped {
        marker: "[1]".into(),
        edge_from: "X".into(),
        edge_to: "Y".into(),
        label: "my label".into(),
    };
    assert_eq!(
        w2.to_string(),
        "Label 'my label' on edge X -> Y moved to legend as [1]"
    );
}

/// Issue #9: No legend when labels fit inline
#[test]
fn test_issue_9_no_legend_when_labels_fit() {
    let input = "flowchart LR\nA[Start] -->|yes| B[End]";
    let result = render_mermaid_to_tui(input, RenderOptions::default()).unwrap();
    assert!(
        !result.output.contains("Labels:"),
        "No legend when labels fit inline"
    );
    assert!(result.output.contains("yes"), "Label should appear inline");
}

// ── Issue #12: Expose supported languages list ───────────────────────

/// Issue #12: SUPPORTED_LANGUAGES contains mermaid and d2
#[test]
fn test_issue_12_supported_languages() {
    assert!(SUPPORTED_LANGUAGES.contains(&"mermaid"));
    assert!(SUPPORTED_LANGUAGES.contains(&"d2"));
}

/// Issue #12: is_supported works case-insensitively
#[test]
fn test_issue_12_is_supported() {
    assert!(is_supported("mermaid"));
    assert!(is_supported("Mermaid"));
    assert!(is_supported("D2"));
    assert!(is_supported("d2"));
    assert!(!is_supported("graphviz"));
    assert!(!is_supported(""));
}

// ── Issue #11: Unified render() entry point ──────────────────────────

/// Issue #11: render("d2", ...) dispatches to D2 parser
#[test]
fn test_issue_11_render_d2() {
    let result = render("d2", "A -> B", RenderOptions::default()).unwrap();
    assert!(result.output.contains("A"));
    assert!(result.output.contains("B"));
}

/// Issue #11: render("mermaid", ...) dispatches to Mermaid auto-detect
#[test]
fn test_issue_11_render_mermaid() {
    let result = render(
        "mermaid",
        "flowchart LR\nA[Start] --> B[End]",
        RenderOptions::default(),
    )
    .unwrap();
    assert!(result.output.contains("Start"));
    assert!(result.output.contains("End"));
}

/// Issue #11: render("mermaid", ...) handles pie charts
#[test]
fn test_issue_11_render_mermaid_pie() {
    let result = render(
        "mermaid",
        "pie\n    \"A\" : 60\n    \"B\" : 40",
        RenderOptions::default(),
    )
    .unwrap();
    assert!(result.output.contains("A"));
    assert!(result.output.contains("60"));
}

/// Issue #11: render is case-insensitive on lang
#[test]
fn test_issue_11_render_case_insensitive() {
    let result = render("D2", "X -> Y", RenderOptions::default()).unwrap();
    assert!(result.output.contains("X"));
    assert!(result.output.contains("Y"));
}

// ── Issue #13: Validate-only check() ─────────────────────────────────

/// Issue #13: check detects cycle without rendering
#[test]
fn test_issue_13_check_cycle() {
    let warnings = check("mermaid", "flowchart LR\nA --> B\nB --> A").unwrap();
    assert!(!warnings.is_empty(), "Should detect cycle");
    assert!(
        matches!(&warnings[0], DiagramWarning::CycleDetected { .. }),
        "Should be CycleDetected"
    );
}

/// Issue #13: check returns empty for valid acyclic graph
#[test]
fn test_issue_13_check_no_warnings() {
    let warnings = check("mermaid", "flowchart LR\nA --> B --> C").unwrap();
    assert!(warnings.is_empty());
}

/// Issue #13: check works with D2
#[test]
fn test_issue_13_check_d2_cycle() {
    let warnings = check("d2", "A -> B\nB -> A").unwrap();
    assert!(!warnings.is_empty());
}

/// Issue #13: check validates pie chart parse errors
#[test]
fn test_issue_13_check_pie_valid() {
    let warnings = check("mermaid", "pie\n    \"A\" : 50\n    \"B\" : 50").unwrap();
    assert!(warnings.is_empty());
}

/// Issue #13: check validates sequence diagram parse
#[test]
fn test_issue_13_check_sequence_valid() {
    let warnings = check("mermaid", "sequenceDiagram\n    A->>B: Hi").unwrap();
    assert!(warnings.is_empty());
}

/// Issue #13: check returns Err on invalid input
#[test]
fn test_issue_13_check_invalid() {
    let result = check("mermaid", "flowchart\n");
    assert!(result.is_err(), "Should fail on invalid input");
}
