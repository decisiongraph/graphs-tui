use std::collections::HashMap;

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

/// A subgraph/group of nodes
#[derive(Debug, Clone)]
pub struct Subgraph {
    pub id: String,
    pub label: String,
    pub nodes: Vec<NodeId>,
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
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        }
    }
}

/// A node in the flowchart
#[derive(Debug, Clone)]
pub struct Node {
    pub id: NodeId,
    pub label: String,
    pub shape: NodeShape,
    pub subgraph: Option<String>,
    pub width: usize,
    pub height: usize,
    pub x: usize,
    pub y: usize,
}

impl Node {
    /// Create a new node with given id and label
    pub fn new(id: NodeId, label: String) -> Self {
        Self {
            id,
            label,
            shape: NodeShape::default(),
            subgraph: None,
            width: 0,
            height: 0,
            x: 0,
            y: 0,
        }
    }

    /// Create a new node with shape
    pub fn with_shape(id: NodeId, label: String, shape: NodeShape) -> Self {
        Self {
            id,
            label,
            shape,
            subgraph: None,
            width: 0,
            height: 0,
            x: 0,
            y: 0,
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
}

impl Graph {
    /// Create a new empty graph with given direction
    pub fn new(direction: Direction) -> Self {
        Self {
            direction,
            nodes: HashMap::new(),
            edges: Vec::new(),
            subgraphs: Vec::new(),
        }
    }
}

/// Options for rendering the diagram
#[derive(Debug, Clone, Default)]
pub struct RenderOptions {
    /// Use ASCII characters instead of Unicode
    pub ascii: bool,
    /// Maximum width (not yet implemented)
    pub max_width: Option<usize>,
}
