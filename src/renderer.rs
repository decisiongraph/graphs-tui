use crate::grid::Grid;
use crate::types::{Direction, Edge, EdgeStyle, Graph, Node, NodeShape, RenderOptions, Subgraph};

/// Unicode box-drawing characters
struct CharSet {
    tl: char,    // top-left corner
    tr: char,    // top-right corner
    bl: char,    // bottom-left corner
    br: char,    // bottom-right corner
    h: char,     // horizontal line
    v: char,     // vertical line
    arr_r: char, // arrow right
    arr_l: char, // arrow left
    arr_d: char, // arrow down
    arr_u: char, // arrow up
    // Rounded corners
    rtl: char,
    rtr: char,
    rbl: char,
    rbr: char,
    // Double lines for subgraphs
    dh: char,
    dv: char,
    dtl: char,
    dtr: char,
    dbl: char,
    dbr: char,
}

const UNICODE_CHARS: CharSet = CharSet {
    tl: '┌',
    tr: '┐',
    bl: '└',
    br: '┘',
    h: '─',
    v: '│',
    arr_r: '▶',
    arr_l: '◀',
    arr_d: '▼',
    arr_u: '▲',
    rtl: '╭',
    rtr: '╮',
    rbl: '╰',
    rbr: '╯',
    dh: '═',
    dv: '║',
    dtl: '╔',
    dtr: '╗',
    dbl: '╚',
    dbr: '╝',
};

const ASCII_CHARS: CharSet = CharSet {
    tl: '+',
    tr: '+',
    bl: '+',
    br: '+',
    h: '-',
    v: '|',
    arr_r: '>',
    arr_l: '<',
    arr_d: 'v',
    arr_u: '^',
    rtl: '+',
    rtr: '+',
    rbl: '+',
    rbr: '+',
    dh: '=',
    dv: '#',
    dtl: '#',
    dtr: '#',
    dbl: '#',
    dbr: '#',
};

/// Render the graph to a string
pub fn render_graph(graph: &Graph, options: &RenderOptions) -> String {
    let chars = if options.ascii {
        &ASCII_CHARS
    } else {
        &UNICODE_CHARS
    };

    // Find grid bounds
    let mut max_x = 0;
    let mut max_y = 0;
    for node in graph.nodes.values() {
        max_x = max_x.max(node.x + node.width);
        max_y = max_y.max(node.y + node.height);
    }
    for sg in &graph.subgraphs {
        max_x = max_x.max(sg.x + sg.width);
        max_y = max_y.max(sg.y + sg.height);
    }

    // Add padding
    let mut grid = Grid::new(max_x + 2, max_y + 2);

    // 1. Render subgraphs first (background)
    for sg in &graph.subgraphs {
        draw_subgraph(&mut grid, sg, chars);
    }

    // 2. Render nodes
    for node in graph.nodes.values() {
        draw_node(&mut grid, node, chars);
    }

    // 3. Render edges
    for edge in &graph.edges {
        if let (Some(from), Some(to)) = (graph.nodes.get(&edge.from), graph.nodes.get(&edge.to)) {
            draw_edge(
                &mut grid,
                from,
                to,
                edge,
                chars,
                graph.direction,
                options.ascii,
            );
        }
    }

    let output = grid.to_string();

    // Apply max_width constraint if set
    if let Some(max_width) = options.max_width {
        output
            .lines()
            .map(|line| {
                if line.len() > max_width {
                    // Truncate and add ellipsis indicator
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
    }
}

/// Draw a subgraph box
fn draw_subgraph(grid: &mut Grid, sg: &Subgraph, chars: &CharSet) {
    if sg.width == 0 || sg.height == 0 {
        return;
    }

    let x = sg.x;
    let y = sg.y;
    let width = sg.width;
    let height = sg.height;

    // Corners (double lines)
    grid.set(x, y, chars.dtl);
    grid.set(x + width - 1, y, chars.dtr);
    grid.set(x, y + height - 1, chars.dbl);
    grid.set(x + width - 1, y + height - 1, chars.dbr);

    // Horizontal lines
    for i in 1..width - 1 {
        grid.set(x + i, y, chars.dh);
        grid.set(x + i, y + height - 1, chars.dh);
    }

    // Vertical lines
    for i in 1..height - 1 {
        grid.set(x, y + i, chars.dv);
        grid.set(x + width - 1, y + i, chars.dv);
    }

    // Label (top center)
    if !sg.label.is_empty() && width > sg.label.len() + 2 {
        let label_x = x + (width - sg.label.len()) / 2;
        for (i, c) in sg.label.chars().enumerate() {
            grid.set(label_x + i, y, c);
        }
    }
}

/// Draw a node with its shape
fn draw_node(grid: &mut Grid, node: &Node, chars: &CharSet) {
    // First, protect the entire node bounding box from edge overwriting
    protect_node_area(grid, node);

    match node.shape {
        NodeShape::Rectangle => draw_rectangle(grid, node, chars),
        NodeShape::Rounded => draw_rounded(grid, node, chars),
        NodeShape::Circle => draw_circle(grid, node, chars),
        NodeShape::Diamond => draw_diamond(grid, node, chars),
        NodeShape::Cylinder => draw_cylinder(grid, node, chars),
        NodeShape::Stadium => draw_stadium(grid, node, chars),
        NodeShape::Subroutine => draw_subroutine(grid, node, chars),
        NodeShape::Hexagon => draw_hexagon(grid, node, chars),
        NodeShape::Parallelogram => draw_parallelogram(grid, node, chars, false),
        NodeShape::ParallelogramAlt => draw_parallelogram(grid, node, chars, true),
        NodeShape::Trapezoid => draw_trapezoid(grid, node, chars, false),
        NodeShape::TrapezoidAlt => draw_trapezoid(grid, node, chars, true),
        NodeShape::Table => draw_table(grid, node, chars),
    }
}

/// Protect the entire node bounding box from being overwritten by edges
fn protect_node_area(grid: &mut Grid, node: &Node) {
    for y in node.y..node.y + node.height {
        for x in node.x..node.x + node.width {
            grid.mark_protected(x, y);
        }
    }
}

/// Draw a rectangle node [Label]
fn draw_rectangle(grid: &mut Grid, node: &Node, chars: &CharSet) {
    let x = node.x;
    let y = node.y;
    let width = node.width;
    let height = node.height;

    // Corners
    grid.set(x, y, chars.tl);
    grid.set(x + width - 1, y, chars.tr);
    grid.set(x, y + height - 1, chars.bl);
    grid.set(x + width - 1, y + height - 1, chars.br);

    // Horizontal lines
    for i in 1..width - 1 {
        grid.set(x + i, y, chars.h);
        grid.set(x + i, y + height - 1, chars.h);
    }

    // Vertical lines
    for i in 1..height - 1 {
        grid.set(x, y + i, chars.v);
        grid.set(x + width - 1, y + i, chars.v);
    }

    // Label (centered)
    draw_label(grid, node);
}

/// Draw a rounded rectangle node (Label)
fn draw_rounded(grid: &mut Grid, node: &Node, chars: &CharSet) {
    let x = node.x;
    let y = node.y;
    let width = node.width;
    let height = node.height;

    // Rounded corners
    grid.set(x, y, chars.rtl);
    grid.set(x + width - 1, y, chars.rtr);
    grid.set(x, y + height - 1, chars.rbl);
    grid.set(x + width - 1, y + height - 1, chars.rbr);

    // Horizontal lines
    for i in 1..width - 1 {
        grid.set(x + i, y, chars.h);
        grid.set(x + i, y + height - 1, chars.h);
    }

    // Vertical lines
    for i in 1..height - 1 {
        grid.set(x, y + i, chars.v);
        grid.set(x + width - 1, y + i, chars.v);
    }

    draw_label(grid, node);
}

/// Draw a circle node ((Label))
fn draw_circle(grid: &mut Grid, node: &Node, chars: &CharSet) {
    let x = node.x;
    let y = node.y;
    let width = node.width;
    let height = node.height;

    // Use rounded corners and parentheses for circle effect
    grid.set(x, y, '(');
    grid.set(x + width - 1, y, ')');
    grid.set(x, y + height - 1, '(');
    grid.set(x + width - 1, y + height - 1, ')');

    // Top/bottom with curves
    for i in 1..width - 1 {
        if i == 1 {
            grid.set(x + i, y, chars.rtl);
            grid.set(x + i, y + height - 1, chars.rbl);
        } else if i == width - 2 {
            grid.set(x + i, y, chars.rtr);
            grid.set(x + i, y + height - 1, chars.rbr);
        } else {
            grid.set(x + i, y, chars.h);
            grid.set(x + i, y + height - 1, chars.h);
        }
    }

    // Sides
    for i in 1..height - 1 {
        grid.set(x, y + i, '(');
        grid.set(x + width - 1, y + i, ')');
    }

    draw_label(grid, node);
}

/// Draw a diamond node {Label}
fn draw_diamond(grid: &mut Grid, node: &Node, chars: &CharSet) {
    let x = node.x;
    let y = node.y;
    let width = node.width;
    let height = node.height;

    // Diamond shape with / and \
    let mid_x = width / 2;

    // Top point
    grid.set(x + mid_x, y, '/');
    if mid_x + 1 < width {
        grid.set(x + mid_x + 1, y, '\\');
    }

    // Bottom point
    grid.set(x + mid_x, y + height - 1, '\\');
    if mid_x + 1 < width {
        grid.set(x + mid_x + 1, y + height - 1, '/');
    }

    // Left and right edges
    for i in 1..height - 1 {
        grid.set(x, y + i, '<');
        grid.set(x + width - 1, y + i, '>');
    }

    // Fill middle row with horizontal line
    for i in 1..width - 1 {
        grid.set(x + i, y + 1, chars.h);
    }

    draw_label(grid, node);
}

/// Draw a cylinder/database node [(Label)]
fn draw_cylinder(grid: &mut Grid, node: &Node, chars: &CharSet) {
    let x = node.x;
    let y = node.y;
    let width = node.width;
    let height = node.height;

    // Top ellipse
    grid.set(x, y, chars.rtl);
    grid.set(x + width - 1, y, chars.rtr);
    for i in 1..width - 1 {
        grid.set(x + i, y, chars.h);
    }

    // Second row (bottom of top ellipse)
    if height > 2 {
        grid.set(x, y + 1, chars.rbl);
        grid.set(x + width - 1, y + 1, chars.rbr);
        for i in 1..width - 1 {
            grid.set(x + i, y + 1, chars.h);
        }
    }

    // Vertical sides
    for i in 2..height - 1 {
        grid.set(x, y + i, chars.v);
        grid.set(x + width - 1, y + i, chars.v);
    }

    // Bottom ellipse
    grid.set(x, y + height - 1, chars.rbl);
    grid.set(x + width - 1, y + height - 1, chars.rbr);
    for i in 1..width - 1 {
        grid.set(x + i, y + height - 1, chars.h);
    }

    // Label in center
    let label_x = x + (width.saturating_sub(node.label.len())) / 2;
    let label_y = y + height / 2;
    for (i, c) in node.label.chars().enumerate() {
        grid.set(label_x + i, label_y, c);
    }
}

/// Draw a stadium node ([Label])
fn draw_stadium(grid: &mut Grid, node: &Node, chars: &CharSet) {
    let x = node.x;
    let y = node.y;
    let width = node.width;
    let height = node.height;

    // Stadium is like rounded but with more pronounced curves
    grid.set(x, y, '(');
    grid.set(x + width - 1, y, ')');
    grid.set(x, y + height - 1, '(');
    grid.set(x + width - 1, y + height - 1, ')');

    // Horizontal lines
    for i in 1..width - 1 {
        grid.set(x + i, y, chars.h);
        grid.set(x + i, y + height - 1, chars.h);
    }

    // Curved sides
    for i in 1..height - 1 {
        grid.set(x, y + i, '(');
        grid.set(x + width - 1, y + i, ')');
    }

    draw_label(grid, node);
}

/// Draw a subroutine node [[Label]]
fn draw_subroutine(grid: &mut Grid, node: &Node, chars: &CharSet) {
    let x = node.x;
    let y = node.y;
    let width = node.width;
    let height = node.height;

    // Double vertical lines on sides
    grid.set(x, y, chars.tl);
    grid.set(x + 1, y, chars.tl);
    grid.set(x + width - 1, y, chars.tr);
    grid.set(x + width - 2, y, chars.tr);

    grid.set(x, y + height - 1, chars.bl);
    grid.set(x + 1, y + height - 1, chars.bl);
    grid.set(x + width - 1, y + height - 1, chars.br);
    grid.set(x + width - 2, y + height - 1, chars.br);

    // Horizontal lines
    for i in 2..width - 2 {
        grid.set(x + i, y, chars.h);
        grid.set(x + i, y + height - 1, chars.h);
    }

    // Double vertical sides
    for i in 1..height - 1 {
        grid.set(x, y + i, chars.v);
        grid.set(x + 1, y + i, chars.v);
        grid.set(x + width - 1, y + i, chars.v);
        grid.set(x + width - 2, y + i, chars.v);
    }

    draw_label(grid, node);
}

/// Draw a hexagon node {{Label}}
fn draw_hexagon(grid: &mut Grid, node: &Node, chars: &CharSet) {
    let x = node.x;
    let y = node.y;
    let width = node.width;
    let height = node.height;

    // Top edge with angled corners
    grid.set(x, y, '/');
    grid.set(x + width - 1, y, '\\');
    for i in 1..width - 1 {
        grid.set(x + i, y, chars.h);
    }

    // Bottom edge with angled corners
    grid.set(x, y + height - 1, '\\');
    grid.set(x + width - 1, y + height - 1, '/');
    for i in 1..width - 1 {
        grid.set(x + i, y + height - 1, chars.h);
    }

    // Sides (angled look with < and >)
    for i in 1..height - 1 {
        grid.set(x, y + i, '<');
        grid.set(x + width - 1, y + i, '>');
    }

    draw_label(grid, node);
}

/// Draw a parallelogram node [/Label/] or [\Label\]
fn draw_parallelogram(grid: &mut Grid, node: &Node, chars: &CharSet, reverse: bool) {
    let x = node.x;
    let y = node.y;
    let width = node.width;
    let height = node.height;

    let (top_left, top_right, bot_left, bot_right) = if reverse {
        ('\\', chars.tr, chars.bl, '/')
    } else {
        (chars.tl, '/', '\\', chars.br)
    };

    // Corners
    grid.set(x, y, top_left);
    grid.set(x + width - 1, y, top_right);
    grid.set(x, y + height - 1, bot_left);
    grid.set(x + width - 1, y + height - 1, bot_right);

    // Horizontal lines
    for i in 1..width - 1 {
        grid.set(x + i, y, chars.h);
        grid.set(x + i, y + height - 1, chars.h);
    }

    // Vertical lines (slanted appearance)
    for i in 1..height - 1 {
        grid.set(x, y + i, if reverse { '\\' } else { '/' });
        grid.set(x + width - 1, y + i, if reverse { '\\' } else { '/' });
    }

    draw_label(grid, node);
}

/// Draw a trapezoid node [/Label\] or [\Label/]
fn draw_trapezoid(grid: &mut Grid, node: &Node, chars: &CharSet, reverse: bool) {
    let x = node.x;
    let y = node.y;
    let width = node.width;
    let height = node.height;

    let (top_left, top_right, bot_left, bot_right) = if reverse {
        ('\\', '/', chars.bl, chars.br)
    } else {
        (chars.tl, chars.tr, '\\', '/')
    };

    // Corners
    grid.set(x, y, top_left);
    grid.set(x + width - 1, y, top_right);
    grid.set(x, y + height - 1, bot_left);
    grid.set(x + width - 1, y + height - 1, bot_right);

    // Horizontal lines
    for i in 1..width - 1 {
        grid.set(x + i, y, chars.h);
        grid.set(x + i, y + height - 1, chars.h);
    }

    // Vertical lines (slanted on one end)
    for i in 1..height - 1 {
        let left_char = if reverse { '\\' } else { chars.v };
        let right_char = if reverse { '/' } else { chars.v };
        grid.set(x, y + i, left_char);
        grid.set(x + width - 1, y + i, right_char);
    }

    draw_label(grid, node);
}

/// Draw a table node (D2 sql_table) - uses double borders
fn draw_table(grid: &mut Grid, node: &Node, chars: &CharSet) {
    let x = node.x;
    let y = node.y;
    let width = node.width;
    let height = node.height;

    // Double-line corners (like subgraph)
    grid.set(x, y, chars.dtl);
    grid.set(x + width - 1, y, chars.dtr);
    grid.set(x, y + height - 1, chars.dbl);
    grid.set(x + width - 1, y + height - 1, chars.dbr);

    // Double horizontal lines
    for i in 1..width - 1 {
        grid.set(x + i, y, chars.dh);
        grid.set(x + i, y + height - 1, chars.dh);
    }

    // Double vertical lines
    for i in 1..height - 1 {
        grid.set(x, y + i, chars.dv);
        grid.set(x + width - 1, y + i, chars.dv);
    }

    draw_label(grid, node);
}

/// Draw the label centered in the node
fn draw_label(grid: &mut Grid, node: &Node) {
    let label_x = node.x + (node.width.saturating_sub(node.label.len())) / 2;
    let label_y = node.y + 1;
    for (i, c) in node.label.chars().enumerate() {
        grid.set(label_x + i, label_y, c);
    }
}

/// Get line characters for edge style
fn get_edge_chars(style: EdgeStyle, chars: &CharSet, ascii: bool) -> (char, char) {
    match style {
        EdgeStyle::Arrow | EdgeStyle::Line => (chars.h, chars.v),
        EdgeStyle::DottedArrow | EdgeStyle::DottedLine => {
            if ascii {
                ('.', ':')
            } else {
                ('·', '·')
            }
        }
        EdgeStyle::ThickArrow | EdgeStyle::ThickLine => (chars.dh, chars.dv),
    }
}

/// Check if edge style has an arrow
fn style_has_arrow(style: EdgeStyle) -> bool {
    matches!(
        style,
        EdgeStyle::Arrow | EdgeStyle::DottedArrow | EdgeStyle::ThickArrow
    )
}

/// Draw an edge between two nodes
fn draw_edge(
    grid: &mut Grid,
    from: &Node,
    to: &Node,
    edge: &Edge,
    chars: &CharSet,
    direction: Direction,
    ascii: bool,
) {
    let has_arrow = style_has_arrow(edge.style);
    let (h_char, v_char) = get_edge_chars(edge.style, chars, ascii);

    let (start_x, start_y, end_x, end_y, arrow_char) = match direction {
        Direction::LR => (
            from.x + from.width,
            from.y + 1,
            to.x,
            to.y + 1,
            if has_arrow { chars.arr_r } else { h_char },
        ),
        Direction::RL => (
            from.x,
            from.y + 1,
            to.x + to.width,
            to.y + 1,
            if has_arrow { chars.arr_l } else { h_char },
        ),
        Direction::TB => (
            from.x + from.width / 2,
            from.y + from.height,
            to.x + to.width / 2,
            to.y,
            if has_arrow { chars.arr_d } else { v_char },
        ),
        Direction::BT => (
            from.x + from.width / 2,
            from.y,
            to.x + to.width / 2,
            to.y + to.height,
            if has_arrow { chars.arr_u } else { v_char },
        ),
    };

    if direction.is_horizontal() {
        draw_horizontal_edge(
            grid,
            start_x,
            start_y,
            end_x,
            end_y,
            h_char,
            v_char,
            arrow_char,
            direction,
            edge.label.as_deref(),
            chars,
        );
    } else {
        draw_vertical_edge(
            grid,
            start_x,
            start_y,
            end_x,
            end_y,
            h_char,
            v_char,
            arrow_char,
            direction,
            edge.label.as_deref(),
            chars,
        );
    }
}

/// Draw edge for LR/RL directions (respects protected node cells)
fn draw_horizontal_edge(
    grid: &mut Grid,
    start_x: usize,
    start_y: usize,
    end_x: usize,
    end_y: usize,
    h_char: char,
    v_char: char,
    arrow_char: char,
    direction: Direction,
    label: Option<&str>,
    chars: &CharSet,
) {
    if start_y == end_y {
        // Straight horizontal line
        let (from_x, to_x) = if end_x > start_x {
            (start_x, end_x)
        } else {
            (end_x, start_x)
        };
        for x in from_x..to_x {
            grid.set_if_empty(x, start_y, h_char);
        }
        // Arrow just before the end
        if end_x > start_x {
            grid.set_if_empty(end_x - 1, end_y, arrow_char);
        } else {
            grid.set_if_empty(end_x + 1, end_y, arrow_char);
        }

        // Draw label in the middle of the edge
        if let Some(lbl) = label {
            let edge_len = to_x.saturating_sub(from_x);
            if edge_len > lbl.len() + 2 {
                let label_x = from_x + (edge_len - lbl.len()) / 2;
                for (i, c) in lbl.chars().enumerate() {
                    grid.set_if_empty(label_x + i, start_y, c);
                }
            }
        }
    } else {
        // L-shaped or Z-shaped routing
        let mid_x = start_x + (end_x.saturating_sub(start_x)) / 2;
        let is_lr = direction == Direction::LR;

        // Horizontal from start to mid
        let (from_x, to_x) = if mid_x > start_x {
            (start_x, mid_x)
        } else {
            (mid_x, start_x)
        };
        for x in from_x..to_x {
            grid.set_if_empty(x, start_y, h_char);
        }

        // Turn 1 at (mid_x, start_y)
        let corner1 = if end_y > start_y {
            if is_lr {
                chars.tr
            } else {
                chars.tl
            }
        } else {
            if is_lr {
                chars.br
            } else {
                chars.bl
            }
        };
        grid.set_if_empty(mid_x, start_y, corner1);

        // Vertical from start_y to end_y
        let (from_y, to_y) = if end_y > start_y {
            (start_y + 1, end_y)
        } else {
            (end_y + 1, start_y)
        };
        for y in from_y..to_y {
            grid.set_if_empty(mid_x, y, v_char);
        }

        // Draw label on vertical segment
        if let Some(lbl) = label {
            let vert_len = to_y.saturating_sub(from_y);
            if vert_len > 0 {
                let label_y = from_y + vert_len / 2;
                // Draw label to the right of the vertical line
                for (i, c) in lbl.chars().enumerate() {
                    grid.set_if_empty(mid_x + 1 + i, label_y, c);
                }
            }
        }

        // Turn 2 at (mid_x, end_y)
        let corner2 = if end_y > start_y {
            if is_lr {
                chars.bl
            } else {
                chars.br
            }
        } else {
            if is_lr {
                chars.tl
            } else {
                chars.tr
            }
        };
        grid.set_if_empty(mid_x, end_y, corner2);

        // Horizontal from mid to end
        let (from_x, to_x) = if end_x > mid_x {
            (mid_x + 1, end_x)
        } else {
            (end_x, mid_x)
        };
        for x in from_x..to_x {
            grid.set_if_empty(x, end_y, h_char);
        }

        // Arrow
        if end_x > mid_x {
            grid.set_if_empty(end_x - 1, end_y, arrow_char);
        } else {
            grid.set_if_empty(end_x + 1, end_y, arrow_char);
        }
    }
}

/// Draw edge for TB/BT directions (respects protected node cells)
fn draw_vertical_edge(
    grid: &mut Grid,
    start_x: usize,
    start_y: usize,
    end_x: usize,
    end_y: usize,
    h_char: char,
    v_char: char,
    arrow_char: char,
    direction: Direction,
    label: Option<&str>,
    chars: &CharSet,
) {
    if start_x == end_x {
        // Straight vertical line
        let (from_y, to_y) = if end_y > start_y {
            (start_y, end_y)
        } else {
            (end_y, start_y)
        };
        for y in from_y..to_y {
            grid.set_if_empty(start_x, y, v_char);
        }
        // Arrow just before the end
        if end_y > start_y {
            grid.set_if_empty(end_x, end_y - 1, arrow_char);
        } else {
            grid.set_if_empty(end_x, end_y + 1, arrow_char);
        }

        // Draw label to the right of the vertical line
        if let Some(lbl) = label {
            let edge_len = to_y.saturating_sub(from_y);
            if edge_len > 0 {
                let label_y = from_y + edge_len / 2;
                for (i, c) in lbl.chars().enumerate() {
                    grid.set_if_empty(start_x + 1 + i, label_y, c);
                }
            }
        }
    } else {
        // L-shaped or Z-shaped routing
        let mid_y = start_y + (end_y.saturating_sub(start_y)) / 2;
        let is_tb = direction == Direction::TB;

        // Vertical from start to mid
        let (from_y, to_y) = if mid_y > start_y {
            (start_y, mid_y)
        } else {
            (mid_y, start_y)
        };
        for y in from_y..to_y {
            grid.set_if_empty(start_x, y, v_char);
        }

        // Turn 1 at (start_x, mid_y)
        let corner1 = if end_x > start_x {
            if is_tb {
                chars.bl
            } else {
                chars.tl
            }
        } else {
            if is_tb {
                chars.br
            } else {
                chars.tr
            }
        };
        grid.set_if_empty(start_x, mid_y, corner1);

        // Horizontal from start_x to end_x
        let (from_x, to_x) = if end_x > start_x {
            (start_x + 1, end_x)
        } else {
            (end_x + 1, start_x)
        };
        for x in from_x..to_x {
            grid.set_if_empty(x, mid_y, h_char);
        }

        // Draw label on horizontal segment
        if let Some(lbl) = label {
            let horiz_len = to_x.saturating_sub(from_x);
            if horiz_len > lbl.len() + 2 {
                let label_x = from_x + (horiz_len - lbl.len()) / 2;
                for (i, c) in lbl.chars().enumerate() {
                    grid.set_if_empty(label_x + i, mid_y, c);
                }
            }
        }

        // Turn 2 at (end_x, mid_y)
        let corner2 = if end_x > start_x {
            if is_tb {
                chars.tr
            } else {
                chars.br
            }
        } else {
            if is_tb {
                chars.tl
            } else {
                chars.bl
            }
        };
        grid.set_if_empty(end_x, mid_y, corner2);

        // Vertical from mid to end
        let (from_y, to_y) = if end_y > mid_y {
            (mid_y + 1, end_y)
        } else {
            (end_y, mid_y)
        };
        for y in from_y..to_y {
            grid.set_if_empty(end_x, y, v_char);
        }

        // Arrow
        if end_y > mid_y {
            grid.set_if_empty(end_x, end_y - 1, arrow_char);
        } else {
            grid.set_if_empty(end_x, end_y + 1, arrow_char);
        }
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
        let output = render_graph(&graph, &RenderOptions::default());
        assert!(output.contains("Start"));
        assert!(output.contains("End"));
        assert!(output.contains("▶"));
    }

    #[test]
    fn test_render_tb() {
        let mut graph = parse_mermaid("flowchart TB\nA[Start] --> B[End]").unwrap();
        compute_layout(&mut graph);
        let output = render_graph(&graph, &RenderOptions::default());
        assert!(output.contains("Start"));
        assert!(output.contains("End"));
        assert!(output.contains("▼"));
    }

    #[test]
    fn test_render_ascii() {
        let mut graph = parse_mermaid("flowchart LR\nA --> B").unwrap();
        compute_layout(&mut graph);
        let output = render_graph(
            &graph,
            &RenderOptions {
                ascii: true,
                max_width: None,
            },
        );
        assert!(output.contains("+---+"));
        assert!(output.contains(">"));
        assert!(!output.contains("┌"));
    }

    #[test]
    fn test_render_rl() {
        let mut graph = parse_mermaid("flowchart RL\nA --> B").unwrap();
        compute_layout(&mut graph);
        let output = render_graph(&graph, &RenderOptions::default());
        assert!(output.contains("◀"));
    }

    #[test]
    fn test_render_bt() {
        let mut graph = parse_mermaid("flowchart BT\nA --> B").unwrap();
        compute_layout(&mut graph);
        let output = render_graph(&graph, &RenderOptions::default());
        assert!(output.contains("▲"));
    }

    #[test]
    fn test_render_rounded() {
        let mut graph = parse_mermaid("flowchart LR\nA(Rounded)").unwrap();
        compute_layout(&mut graph);
        let output = render_graph(&graph, &RenderOptions::default());
        assert!(output.contains("Rounded"));
        assert!(output.contains("╭")); // Rounded corner
    }

    #[test]
    fn test_render_circle() {
        let mut graph = parse_mermaid("flowchart LR\nA((Circle))").unwrap();
        compute_layout(&mut graph);
        let output = render_graph(&graph, &RenderOptions::default());
        assert!(output.contains("Circle"));
        assert!(output.contains("(")); // Circle sides
    }

    #[test]
    fn test_render_diamond() {
        let mut graph = parse_mermaid("flowchart LR\nA{Decision}").unwrap();
        compute_layout(&mut graph);
        let output = render_graph(&graph, &RenderOptions::default());
        assert!(output.contains("Decision"));
        assert!(output.contains("<")); // Diamond sides
    }

    #[test]
    fn test_render_cylinder() {
        let mut graph = parse_mermaid("flowchart LR\nDB[(Database)]").unwrap();
        compute_layout(&mut graph);
        let output = render_graph(&graph, &RenderOptions::default());
        assert!(output.contains("Database"));
    }

    #[test]
    fn test_render_max_width() {
        let mut graph = parse_mermaid("flowchart LR\nA[Start] --> B[End]").unwrap();
        compute_layout(&mut graph);
        let output = render_graph(
            &graph,
            &RenderOptions {
                ascii: false,
                max_width: Some(20),
            },
        );
        // All lines should be truncated to max_width
        for line in output.lines() {
            assert!(
                line.chars().count() <= 20,
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
        let output = render_graph(
            &graph,
            &RenderOptions {
                ascii: false,
                max_width: Some(100), // Wide enough to not truncate
            },
        );
        // Should not contain ellipsis when no truncation needed
        assert!(!output.contains('…'));
    }
}
