#![allow(clippy::too_many_arguments)]
#![allow(clippy::collapsible_else_if)]

//! graphs-tui - Terminal renderer for Mermaid and D2 diagrams
//!
//! # Mermaid Flowchart Example
//! ```
//! use graphs_tui::{render_mermaid_to_tui, RenderOptions};
//!
//! let input = "flowchart LR\nA[Start] --> B[End]";
//! let output = render_mermaid_to_tui(input, RenderOptions::default()).unwrap();
//! println!("{}", output);
//! ```
//!
//! # State Diagram Example
//! ```
//! use graphs_tui::{render_state_diagram, RenderOptions};
//!
//! let input = "stateDiagram-v2\n    [*] --> Idle\n    Idle --> Running";
//! let output = render_state_diagram(input, RenderOptions::default()).unwrap();
//! println!("{}", output);
//! ```
//!
//! # Pie Chart Example
//! ```
//! use graphs_tui::{render_pie_chart, RenderOptions};
//!
//! let input = "pie\n    \"Chrome\" : 65\n    \"Firefox\" : 35";
//! let output = render_pie_chart(input, RenderOptions::default()).unwrap();
//! println!("{}", output);
//! ```
//!
//! # D2 Example
//! ```
//! use graphs_tui::{render_d2_to_tui, RenderOptions};
//!
//! let input = "A -> B: connection";
//! let output = render_d2_to_tui(input, RenderOptions::default()).unwrap();
//! println!("{}", output);
//! ```
//!
//! # Sequence Diagram Example
//! ```
//! use graphs_tui::{render_sequence_diagram, RenderOptions};
//!
//! let input = "sequenceDiagram\n    Alice->>Bob: Hello\n    Bob-->>Alice: Hi!";
//! let output = render_sequence_diagram(input, RenderOptions::default()).unwrap();
//! println!("{}", output);
//! ```
//!
//! # Auto-detect Format
//! ```
//! use graphs_tui::{render_diagram, RenderOptions};
//!
//! let mermaid_input = "flowchart LR\nA --> B";
//! let d2_input = "A -> B";
//!
//! // Automatically detects format
//! let _ = render_diagram(mermaid_input, RenderOptions::default());
//! let _ = render_diagram(d2_input, RenderOptions::default());
//! ```

mod d2_parser;
mod error;
mod grid;
mod layout;
mod parser;
mod pie_parser;
mod renderer;
mod seq_parser;
mod state_parser;
mod types;

pub use error::MermaidError;
pub use types::{
    Direction, Edge, EdgeStyle, Graph, Node, NodeId, NodeShape, RenderOptions, Subgraph,
};

use d2_parser::parse_d2;
use layout::compute_layout;
use parser::parse_mermaid;
use pie_parser::{parse_pie_chart as parse_pie, render_pie_chart as render_pie};
use renderer::render_graph;
use seq_parser::{parse_sequence_diagram as parse_seq, render_sequence_diagram as render_seq};
use state_parser::parse_state_diagram;

/// Diagram format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagramFormat {
    /// Mermaid flowchart syntax
    Mermaid,
    /// Mermaid state diagram
    StateDiagram,
    /// Mermaid sequence diagram
    SequenceDiagram,
    /// Mermaid pie chart
    PieChart,
    /// D2 diagram language
    D2,
}

/// Detect the diagram format from input
pub fn detect_format(input: &str) -> DiagramFormat {
    let trimmed = input.trim();
    let lower = trimmed.to_lowercase();

    // Check for specific diagram types first
    if lower.starts_with("sequencediagram") {
        return DiagramFormat::SequenceDiagram;
    }
    if lower.starts_with("statediagram") {
        return DiagramFormat::StateDiagram;
    }
    if lower.starts_with("pie") {
        return DiagramFormat::PieChart;
    }

    // Mermaid flowchart indicators
    if trimmed.starts_with("flowchart")
        || trimmed.starts_with("graph ")
        || trimmed.contains("-->")
        || trimmed.contains("-.-")
        || trimmed.contains("==>")
    {
        return DiagramFormat::Mermaid;
    }

    // D2 uses different arrow syntax
    // D2: ->, <-, <->, --
    // Mermaid: -->, <--, <-->, ---

    DiagramFormat::D2
}

/// Render diagram with auto-detection of format
///
/// # Arguments
/// * `input` - Diagram syntax string (Mermaid, State, Pie, or D2)
/// * `options` - Rendering options
///
/// # Returns
/// * `Ok(String)` - Rendered diagram as string
/// * `Err(MermaidError)` - Parse or layout error
pub fn render_diagram(input: &str, options: RenderOptions) -> Result<String, MermaidError> {
    match detect_format(input) {
        DiagramFormat::Mermaid => render_mermaid_to_tui(input, options),
        DiagramFormat::StateDiagram => render_state_diagram(input, options),
        DiagramFormat::SequenceDiagram => render_sequence_diagram(input, options),
        DiagramFormat::PieChart => render_pie_chart(input, options),
        DiagramFormat::D2 => render_d2_to_tui(input, options),
    }
}

/// Render mermaid flowchart syntax to terminal-displayable text
///
/// # Arguments
/// * `input` - Mermaid flowchart syntax string
/// * `options` - Rendering options (ASCII mode, max width)
///
/// # Returns
/// * `Ok(String)` - Rendered diagram as string
/// * `Err(MermaidError)` - Parse or layout error
pub fn render_mermaid_to_tui(input: &str, options: RenderOptions) -> Result<String, MermaidError> {
    let mut graph = parse_mermaid(input)?;
    compute_layout(&mut graph);
    Ok(render_graph(&graph, &options))
}

/// Render mermaid state diagram to terminal-displayable text
///
/// # Arguments
/// * `input` - Mermaid state diagram syntax string
/// * `options` - Rendering options (ASCII mode, max width)
///
/// # Returns
/// * `Ok(String)` - Rendered diagram as string
/// * `Err(MermaidError)` - Parse or layout error
pub fn render_state_diagram(input: &str, options: RenderOptions) -> Result<String, MermaidError> {
    let mut graph = parse_state_diagram(input)?;
    compute_layout(&mut graph);
    Ok(render_graph(&graph, &options))
}

/// Render mermaid pie chart to terminal-displayable text
///
/// Pie charts are rendered as horizontal bar charts in terminal.
///
/// # Arguments
/// * `input` - Mermaid pie chart syntax string
/// * `options` - Rendering options
///
/// # Returns
/// * `Ok(String)` - Rendered chart as string
/// * `Err(MermaidError)` - Parse error
pub fn render_pie_chart(input: &str, options: RenderOptions) -> Result<String, MermaidError> {
    let chart = parse_pie(input)?;
    Ok(render_pie(&chart, &options))
}

/// Render D2 diagram syntax to terminal-displayable text
///
/// # Arguments
/// * `input` - D2 diagram syntax string
/// * `options` - Rendering options (ASCII mode, max width)
///
/// # Returns
/// * `Ok(String)` - Rendered diagram as string
/// * `Err(MermaidError)` - Parse or layout error
pub fn render_d2_to_tui(input: &str, options: RenderOptions) -> Result<String, MermaidError> {
    let mut graph = parse_d2(input)?;
    compute_layout(&mut graph);
    Ok(render_graph(&graph, &options))
}

/// Render mermaid sequence diagram to terminal-displayable text
///
/// # Arguments
/// * `input` - Mermaid sequence diagram syntax string
/// * `options` - Rendering options (ASCII mode, max width)
///
/// # Returns
/// * `Ok(String)` - Rendered diagram as string
/// * `Err(MermaidError)` - Parse error
pub fn render_sequence_diagram(
    input: &str,
    options: RenderOptions,
) -> Result<String, MermaidError> {
    let diagram = parse_seq(input)?;
    Ok(render_seq(&diagram, &options))
}
