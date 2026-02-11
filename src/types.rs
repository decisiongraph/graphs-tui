use std::collections::HashMap;
use std::fmt;

/// Node identifier type
pub type NodeId = String;

/// Flow direction for the diagram
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Left to Right
    LR,
    /// Right to Left
    RL,
    /// Top to Bottom
    TB,
    /// Bottom to Top
    BT,
}

impl Direction {
    /// Parse direction from string
    pub fn parse(s: &str) -> Option<Direction> {
        match s.to_uppercase().as_str() {
            "LR" => Some(Direction::LR),
            "RL" => Some(Direction::RL),
            "TB" | "TD" => Some(Direction::TB),
            "BT" => Some(Direction::BT),
            _ => None,
        }
    }

    /// Check if direction is horizontal (LR or RL)
    pub fn is_horizontal(&self) -> bool {
        matches!(self, Direction::LR | Direction::RL)
    }
}

/// Shape of a node
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NodeShape {
    /// Rectangle [Label]
    #[default]
    Rectangle,
    /// Rounded rectangle (Label)
    Rounded,
    /// Circle ((Label))
    Circle,
    /// Diamond/rhombus {Label}
    Diamond,
    /// Cylinder/database [(Label)]
    Cylinder,
    /// Stadium shape ([Label])
    Stadium,
    /// Subroutine [[Label]]
    Subroutine,
    /// Hexagon {{Label}}
    Hexagon,
    /// Parallelogram [/Label/]
    Parallelogram,
    /// Reverse Parallelogram [\Label\]
    ParallelogramAlt,
    /// Trapezoid [/Label\]
    Trapezoid,
    /// Reverse Trapezoid [\Label/]
    TrapezoidAlt,
    /// Table (D2 sql_table)
    Table,
    /// Person (D2 stick figure)
    Person,
    /// Cloud (D2 bumpy border)
    Cloud,
    /// Document/page (D2 wavy bottom)
    Document,
}

/// Style of an edge/link
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EdgeStyle {
    /// Solid arrow -->
    #[default]
    Arrow,
    /// Solid line ---
    Line,
    /// Dotted arrow -.->
    DottedArrow,
    /// Dotted line -.-
    DottedLine,
    /// Thick arrow ==>
    ThickArrow,
    /// Thick line ===
    ThickLine,
}

/// A field inside a sql_table or class node (D2)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableField {
    pub name: String,
    pub type_info: Option<String>,
    pub constraint: Option<String>,
}

/// A subgraph/group of nodes
#[derive(Debug, Clone)]
pub struct Subgraph {
    pub id: String,
    pub label: String,
    pub nodes: Vec<NodeId>,
    pub parent: Option<String>,
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}

impl Subgraph {
    pub fn new(id: String, label: String) -> Self {
        Self {
            id,
            label,
            nodes: Vec::new(),
            parent: None,
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        }
    }
}

/// ANSI color for styling
#[derive(Debug, Clone, Default)]
pub struct NodeStyle {
    /// Foreground color (ANSI escape code)
    pub color: Option<String>,
}

/// A node in the flowchart
#[derive(Debug, Clone)]
pub struct Node {
    pub id: NodeId,
    pub label: String,
    pub shape: NodeShape,
    pub subgraph: Option<String>,
    pub fields: Vec<TableField>,
    pub width: usize,
    pub height: usize,
    pub x: usize,
    pub y: usize,
    /// Style class name applied to this node
    pub style_class: Option<String>,
}

impl Node {
    /// Create a new node with given id and label
    pub fn new(id: NodeId, label: String) -> Self {
        Self {
            id,
            label,
            shape: NodeShape::default(),
            subgraph: None,
            fields: Vec::new(),
            width: 0,
            height: 0,
            x: 0,
            y: 0,
            style_class: None,
        }
    }

    /// Create a new node with shape
    pub fn with_shape(id: NodeId, label: String, shape: NodeShape) -> Self {
        Self {
            id,
            label,
            shape,
            subgraph: None,
            fields: Vec::new(),
            width: 0,
            height: 0,
            x: 0,
            y: 0,
            style_class: None,
        }
    }
}

/// An edge connecting two nodes
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edge {
    pub from: NodeId,
    pub to: NodeId,
    pub label: Option<String>,
    pub style: EdgeStyle,
}

/// The complete graph structure
#[derive(Debug, Clone)]
pub struct Graph {
    pub direction: Direction,
    pub nodes: HashMap<NodeId, Node>,
    pub edges: Vec<Edge>,
    pub subgraphs: Vec<Subgraph>,
    /// Style class definitions (classDef name color:#hex)
    pub style_classes: HashMap<String, NodeStyle>,
}

impl Graph {
    /// Create a new empty graph with given direction
    pub fn new(direction: Direction) -> Self {
        Self {
            direction,
            nodes: HashMap::new(),
            edges: Vec::new(),
            subgraphs: Vec::new(),
            style_classes: HashMap::new(),
        }
    }
}

/// Options for rendering the diagram
#[derive(Debug, Clone)]
pub struct RenderOptions {
    /// Use ASCII characters instead of Unicode
    pub ascii: bool,
    /// Maximum width constraint for the diagram
    pub max_width: Option<usize>,
    /// Horizontal gap between nodes (default: 8)
    pub padding_x: usize,
    /// Vertical gap between nodes (default: 4)
    pub padding_y: usize,
    /// Padding between text and node border (default: 1)
    pub border_padding: usize,
    /// Enable ANSI color output (default: false)
    pub colors: bool,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            ascii: false,
            max_width: None,
            padding_x: 8,
            padding_y: 4,
            border_padding: 1,
            colors: false,
        }
    }
}

/// Structured warning emitted during layout or rendering
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagramWarning {
    /// A cycle was detected involving the listed nodes
    CycleDetected { nodes: Vec<String> },
    /// An edge label was too long to render inline and was moved to a legend
    LabelDropped {
        marker: String,
        edge_from: String,
        edge_to: String,
        label: String,
    },
    /// A D2 feature is not supported in TUI rendering
    UnsupportedFeature { feature: String, line: usize },
}

impl fmt::Display for DiagramWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiagramWarning::CycleDetected { nodes } => {
                write!(f, "Cycle detected involving nodes: {}", nodes.join(", "))
            }
            DiagramWarning::LabelDropped {
                marker,
                edge_from,
                edge_to,
                label,
            } => {
                write!(
                    f,
                    "Label '{}' on edge {} -> {} moved to legend as {}",
                    label, edge_from, edge_to, marker
                )
            }
            DiagramWarning::UnsupportedFeature { feature, line } => {
                write!(f, "Unsupported D2 feature '{}' on line {}", feature, line)
            }
        }
    }
}

/// Result of rendering a diagram
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderResult {
    /// The rendered diagram output
    pub output: String,
    /// Warnings generated during layout/rendering
    pub warnings: Vec<DiagramWarning>,
}
