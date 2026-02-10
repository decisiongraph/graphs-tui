//! Tests for GitHub issue #7
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
    assert!(!result.warnings.is_empty(), "Cycle should produce a warning");
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
