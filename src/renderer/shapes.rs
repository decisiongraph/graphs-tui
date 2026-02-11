//! Shape drawing functions for nodes

use crate::grid::Grid;
use crate::text::display_width;
use crate::types::{Node, NodeShape};
use unicode_width::UnicodeWidthChar;

use super::charset::CharSet;

/// Draw a node with its shape
pub fn draw_node(grid: &mut Grid, node: &Node, chars: &CharSet) {
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
        NodeShape::Person => draw_person(grid, node, chars),
        NodeShape::Cloud => draw_cloud(grid, node, chars),
        NodeShape::Document => draw_document(grid, node, chars),
    }

    // Protect the node bounding box from edge overwriting
    protect_node_area(grid, node);
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
    grid.set_if_empty(x, y, chars.tl);
    grid.set_if_empty(x + width - 1, y, chars.tr);
    grid.set_if_empty(x, y + height - 1, chars.bl);
    grid.set_if_empty(x + width - 1, y + height - 1, chars.br);

    // Horizontal lines
    for i in 1..width - 1 {
        grid.set_if_empty(x + i, y, chars.h);
        grid.set_if_empty(x + i, y + height - 1, chars.h);
    }

    // Vertical lines
    for i in 1..height - 1 {
        grid.set_if_empty(x, y + i, chars.v);
        grid.set_if_empty(x + width - 1, y + i, chars.v);
    }

    draw_label(grid, node);
}

/// Draw a rounded rectangle node (Label)
fn draw_rounded(grid: &mut Grid, node: &Node, chars: &CharSet) {
    let x = node.x;
    let y = node.y;
    let width = node.width;
    let height = node.height;

    // Rounded corners
    grid.set_if_empty(x, y, chars.rtl);
    grid.set_if_empty(x + width - 1, y, chars.rtr);
    grid.set_if_empty(x, y + height - 1, chars.rbl);
    grid.set_if_empty(x + width - 1, y + height - 1, chars.rbr);

    // Horizontal lines
    for i in 1..width - 1 {
        grid.set_if_empty(x + i, y, chars.h);
        grid.set_if_empty(x + i, y + height - 1, chars.h);
    }

    // Vertical lines
    for i in 1..height - 1 {
        grid.set_if_empty(x, y + i, chars.v);
        grid.set_if_empty(x + width - 1, y + i, chars.v);
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
    grid.set_if_empty(x, y, '(');
    grid.set_if_empty(x + width - 1, y, ')');
    grid.set_if_empty(x, y + height - 1, '(');
    grid.set_if_empty(x + width - 1, y + height - 1, ')');

    // Top/bottom with curves
    for i in 1..width - 1 {
        if i == 1 {
            grid.set_if_empty(x + i, y, chars.rtl);
            grid.set_if_empty(x + i, y + height - 1, chars.rbl);
        } else if i == width - 2 {
            grid.set_if_empty(x + i, y, chars.rtr);
            grid.set_if_empty(x + i, y + height - 1, chars.rbr);
        } else {
            grid.set_if_empty(x + i, y, chars.h);
            grid.set_if_empty(x + i, y + height - 1, chars.h);
        }
    }

    // Sides
    for i in 1..height - 1 {
        grid.set_if_empty(x, y + i, '(');
        grid.set_if_empty(x + width - 1, y + i, ')');
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
    grid.set_if_empty(x + mid_x, y, '/');
    if mid_x + 1 < width {
        grid.set_if_empty(x + mid_x + 1, y, '\\');
    }

    // Bottom point
    grid.set_if_empty(x + mid_x, y + height - 1, '\\');
    if mid_x + 1 < width {
        grid.set_if_empty(x + mid_x + 1, y + height - 1, '/');
    }

    // Left and right edges
    for i in 1..height - 1 {
        grid.set_if_empty(x, y + i, '<');
        grid.set_if_empty(x + width - 1, y + i, '>');
    }

    // Fill middle row with horizontal line
    for i in 1..width - 1 {
        grid.set_if_empty(x + i, y + 1, chars.h);
    }

    draw_label(grid, node);
}

/// Draw a cylinder/database node [(Label)]
/// 5-row layout:
///   ╭───╮  row 0: rtl + h + rtr
///   ├───┤  row 1: ml + h + mr
///   │ X │  row 2: v + label + v
///   ├───┤  row 3: ml + h + mr
///   ╰───╯  row 4: rbl + h + rbr
fn draw_cylinder(grid: &mut Grid, node: &Node, chars: &CharSet) {
    let x = node.x;
    let y = node.y;
    let width = node.width;
    let height = node.height;

    // Row 0: top rounded
    grid.set_if_empty(x, y, chars.rtl);
    grid.set_if_empty(x + width - 1, y, chars.rtr);
    for i in 1..width - 1 {
        grid.set_if_empty(x + i, y, chars.h);
    }

    // Row 1: top separator
    grid.set_if_empty(x, y + 1, chars.ml);
    grid.set_if_empty(x + width - 1, y + 1, chars.mr);
    for i in 1..width - 1 {
        grid.set_if_empty(x + i, y + 1, chars.h);
    }

    // Middle rows: vertical sides
    for i in 2..height - 2 {
        grid.set_if_empty(x, y + i, chars.v);
        grid.set_if_empty(x + width - 1, y + i, chars.v);
    }

    // Row height-2: bottom separator
    grid.set_if_empty(x, y + height - 2, chars.ml);
    grid.set_if_empty(x + width - 1, y + height - 2, chars.mr);
    for i in 1..width - 1 {
        grid.set_if_empty(x + i, y + height - 2, chars.h);
    }

    // Row height-1: bottom rounded
    grid.set_if_empty(x, y + height - 1, chars.rbl);
    grid.set_if_empty(x + width - 1, y + height - 1, chars.rbr);
    for i in 1..width - 1 {
        grid.set_if_empty(x + i, y + height - 1, chars.h);
    }

    draw_label(grid, node);
}

/// Draw a stadium node ([Label])
fn draw_stadium(grid: &mut Grid, node: &Node, chars: &CharSet) {
    let x = node.x;
    let y = node.y;
    let width = node.width;
    let height = node.height;

    // Stadium is like rounded but with more pronounced curves
    grid.set_if_empty(x, y, '(');
    grid.set_if_empty(x + width - 1, y, ')');
    grid.set_if_empty(x, y + height - 1, '(');
    grid.set_if_empty(x + width - 1, y + height - 1, ')');

    // Horizontal lines
    for i in 1..width - 1 {
        grid.set_if_empty(x + i, y, chars.h);
        grid.set_if_empty(x + i, y + height - 1, chars.h);
    }

    // Curved sides
    for i in 1..height - 1 {
        grid.set_if_empty(x, y + i, '(');
        grid.set_if_empty(x + width - 1, y + i, ')');
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
    grid.set_if_empty(x, y, chars.tl);
    grid.set_if_empty(x + 1, y, chars.tl);
    grid.set_if_empty(x + width - 1, y, chars.tr);
    grid.set_if_empty(x + width - 2, y, chars.tr);

    grid.set_if_empty(x, y + height - 1, chars.bl);
    grid.set_if_empty(x + 1, y + height - 1, chars.bl);
    grid.set_if_empty(x + width - 1, y + height - 1, chars.br);
    grid.set_if_empty(x + width - 2, y + height - 1, chars.br);

    // Horizontal lines
    for i in 2..width - 2 {
        grid.set_if_empty(x + i, y, chars.h);
        grid.set_if_empty(x + i, y + height - 1, chars.h);
    }

    // Double vertical sides
    for i in 1..height - 1 {
        grid.set_if_empty(x, y + i, chars.v);
        grid.set_if_empty(x + 1, y + i, chars.v);
        grid.set_if_empty(x + width - 1, y + i, chars.v);
        grid.set_if_empty(x + width - 2, y + i, chars.v);
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
    grid.set_if_empty(x, y, '/');
    grid.set_if_empty(x + width - 1, y, '\\');
    for i in 1..width - 1 {
        grid.set_if_empty(x + i, y, chars.h);
    }

    // Bottom edge with angled corners
    grid.set_if_empty(x, y + height - 1, '\\');
    grid.set_if_empty(x + width - 1, y + height - 1, '/');
    for i in 1..width - 1 {
        grid.set_if_empty(x + i, y + height - 1, chars.h);
    }

    // Sides (angled look with < and >)
    for i in 1..height - 1 {
        grid.set_if_empty(x, y + i, '<');
        grid.set_if_empty(x + width - 1, y + i, '>');
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
    grid.set_if_empty(x, y, top_left);
    grid.set_if_empty(x + width - 1, y, top_right);
    grid.set_if_empty(x, y + height - 1, bot_left);
    grid.set_if_empty(x + width - 1, y + height - 1, bot_right);

    // Horizontal lines
    for i in 1..width - 1 {
        grid.set_if_empty(x + i, y, chars.h);
        grid.set_if_empty(x + i, y + height - 1, chars.h);
    }

    // Vertical lines (slanted appearance)
    for i in 1..height - 1 {
        grid.set_if_empty(x, y + i, if reverse { '\\' } else { '/' });
        grid.set_if_empty(x + width - 1, y + i, if reverse { '\\' } else { '/' });
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
    grid.set_if_empty(x, y, top_left);
    grid.set_if_empty(x + width - 1, y, top_right);
    grid.set_if_empty(x, y + height - 1, bot_left);
    grid.set_if_empty(x + width - 1, y + height - 1, bot_right);

    // Horizontal lines
    for i in 1..width - 1 {
        grid.set_if_empty(x + i, y, chars.h);
        grid.set_if_empty(x + i, y + height - 1, chars.h);
    }

    // Vertical lines (slanted on one end)
    for i in 1..height - 1 {
        let left_char = if reverse { '\\' } else { chars.v };
        let right_char = if reverse { '/' } else { chars.v };
        grid.set_if_empty(x, y + i, left_char);
        grid.set_if_empty(x + width - 1, y + i, right_char);
    }

    draw_label(grid, node);
}

/// Draw a table node (D2 sql_table) - uses double borders with field rows
fn draw_table(grid: &mut Grid, node: &Node, chars: &CharSet) {
    let x = node.x;
    let y = node.y;
    let width = node.width;
    let height = node.height;

    if node.fields.is_empty() {
        // Simple table without fields (original behavior)
        grid.set_if_empty(x, y, chars.dtl);
        grid.set_if_empty(x + width - 1, y, chars.dtr);
        grid.set_if_empty(x, y + height - 1, chars.dbl);
        grid.set_if_empty(x + width - 1, y + height - 1, chars.dbr);

        for i in 1..width - 1 {
            grid.set_if_empty(x + i, y, chars.dh);
            grid.set_if_empty(x + i, y + height - 1, chars.dh);
        }

        for i in 1..height - 1 {
            grid.set_if_empty(x, y + i, chars.dv);
            grid.set_if_empty(x + width - 1, y + i, chars.dv);
        }

        draw_label(grid, node);
        return;
    }

    // Table with fields layout:
    // Row 0: ╔═══════════╗  top border
    // Row 1: ║   label   ║  label row
    // Row 2: ╠═══════════╣  separator (using ╠/╣ for T-junctions)
    // Row 3: ║ field 1   ║  field rows...
    // Row N: ╚═══════════╝  bottom border

    // Top border
    grid.set_if_empty(x, y, chars.dtl);
    grid.set_if_empty(x + width - 1, y, chars.dtr);
    for i in 1..width - 1 {
        grid.set_if_empty(x + i, y, chars.dh);
    }

    // Label row (row 1) - use first line for table header
    grid.set_if_empty(x, y + 1, chars.dv);
    grid.set_if_empty(x + width - 1, y + 1, chars.dv);
    // Center label (use first line if multi-line)
    let first_line = node.label.split('\n').next().unwrap_or(&node.label);
    let label_x = x + (width.saturating_sub(display_width(first_line))) / 2;
    draw_text(grid, label_x, y + 1, first_line);

    // Separator (row 2)
    grid.set_if_empty(x, y + 2, chars.ml);
    grid.set_if_empty(x + width - 1, y + 2, chars.mr);
    for i in 1..width - 1 {
        grid.set_if_empty(x + i, y + 2, chars.h);
    }

    // Field rows
    for (fi, field) in node.fields.iter().enumerate() {
        let row_y = y + 3 + fi;
        grid.set_if_empty(x, row_y, chars.v);
        grid.set_if_empty(x + width - 1, row_y, chars.v);

        // Format field text
        let field_text = format_field_text(field, width.saturating_sub(4));
        let text_x = x + 2; // 1 for border + 1 padding
        draw_text(grid, text_x, row_y, &field_text);
    }

    // Bottom border
    let bot_y = y + height - 1;
    grid.set_if_empty(x, bot_y, chars.bl);
    grid.set_if_empty(x + width - 1, bot_y, chars.br);
    for i in 1..width - 1 {
        grid.set_if_empty(x + i, bot_y, chars.h);
    }
}

/// Format a table field for display
fn format_field_text(field: &crate::types::TableField, max_width: usize) -> String {
    let mut text = field.name.clone();
    if let Some(ref ti) = field.type_info {
        text.push_str(": ");
        text.push_str(ti);
    }
    if let Some(ref c) = field.constraint {
        let abbrev = match c.as_str() {
            "primary_key" => " [PK]",
            "foreign_key" => " [FK]",
            "unique" => " [UQ]",
            "not_null" => " [NN]",
            other => {
                text.push_str(" [");
                text.push_str(other);
                text.push(']');
                if display_width(&text) > max_width {
                    truncate_to_width(&mut text, max_width);
                }
                return text;
            }
        };
        text.push_str(abbrev);
    }
    if display_width(&text) > max_width {
        truncate_to_width(&mut text, max_width);
    }
    text
}

/// Truncate string to fit within display width
fn truncate_to_width(s: &mut String, max_width: usize) {
    let mut width = 0;
    let mut byte_pos = 0;
    for c in s.chars() {
        let cw = UnicodeWidthChar::width(c).unwrap_or(1);
        if width + cw > max_width {
            break;
        }
        width += cw;
        byte_pos += c.len_utf8();
    }
    s.truncate(byte_pos);
}

/// Draw a person/stick figure node (D2 person shape)
///
/// Layout (min 5 rows, 7 cols):
///    O       head
///   /|\      torso
///   / \      legs
///  Label     label below body
fn draw_person(grid: &mut Grid, node: &Node, chars: &CharSet) {
    let x = node.x;
    let y = node.y;
    let width = node.width;
    let height = node.height;

    let mid_x = x + width / 2;

    // Head
    grid.set_if_empty(mid_x, y, 'O');

    // Torso
    if y + 1 < y + height {
        if mid_x > 0 {
            grid.set_if_empty(mid_x - 1, y + 1, '/');
        }
        grid.set_if_empty(mid_x, y + 1, '|');
        grid.set_if_empty(mid_x + 1, y + 1, '\\');
    }

    // Legs
    if y + 2 < y + height {
        if mid_x > 0 {
            grid.set_if_empty(mid_x - 1, y + 2, '/');
        }
        grid.set_if_empty(mid_x + 1, y + 2, '\\');
    }

    // Label centered below the figure
    let label_lines: Vec<&str> = node.label.split('\n').collect();
    let label_start_y = y + 3;
    for (li, line) in label_lines.iter().enumerate() {
        let lw = display_width(line);
        let lx = x + (width.saturating_sub(lw)) / 2;
        let ly = label_start_y + li;
        if ly < y + height {
            draw_text(grid, lx, ly, line);
        }
    }

    // Draw border box around the whole thing
    grid.set_if_empty(x, y, chars.rtl);
    grid.set_if_empty(x + width - 1, y, chars.rtr);
    grid.set_if_empty(x, y + height - 1, chars.rbl);
    grid.set_if_empty(x + width - 1, y + height - 1, chars.rbr);
    for i in 1..width - 1 {
        grid.set_if_empty(x + i, y, chars.h);
        grid.set_if_empty(x + i, y + height - 1, chars.h);
    }
    for i in 1..height - 1 {
        grid.set_if_empty(x, y + i, chars.v);
        grid.set_if_empty(x + width - 1, y + i, chars.v);
    }
}

/// Draw a cloud node (D2 cloud shape)
///
/// Bumpy border effect:
///   .-~~~-.
///  (       )
///  ( Label )
///   `~---~'
fn draw_cloud(grid: &mut Grid, node: &Node, _chars: &CharSet) {
    let x = node.x;
    let y = node.y;
    let width = node.width;
    let height = node.height;

    // Top row: bumpy
    grid.set_if_empty(x + 1, y, '.');
    grid.set_if_empty(x + 2, y, '-');
    for i in 3..width.saturating_sub(3) {
        grid.set_if_empty(x + i, y, '~');
    }
    if width > 4 {
        grid.set_if_empty(x + width - 3, y, '-');
    }
    grid.set_if_empty(x + width - 2, y, '.');

    // Bottom row: bumpy
    grid.set_if_empty(x + 1, y + height - 1, '`');
    grid.set_if_empty(x + 2, y + height - 1, '~');
    for i in 3..width.saturating_sub(3) {
        grid.set_if_empty(x + i, y + height - 1, '-');
    }
    if width > 4 {
        grid.set_if_empty(x + width - 3, y + height - 1, '~');
    }
    grid.set_if_empty(x + width - 2, y + height - 1, '\'');

    // Sides
    for i in 1..height - 1 {
        grid.set_if_empty(x, y + i, '(');
        grid.set_if_empty(x + width - 1, y + i, ')');
    }

    draw_label(grid, node);
}

/// Draw a document node (D2 document/page shape)
///
/// Rectangle with wavy bottom:
/// ┌───────┐
/// │ Label │
/// │       │
/// └~──────┘
fn draw_document(grid: &mut Grid, node: &Node, chars: &CharSet) {
    let x = node.x;
    let y = node.y;
    let width = node.width;
    let height = node.height;

    // Top corners and line
    grid.set_if_empty(x, y, chars.tl);
    grid.set_if_empty(x + width - 1, y, chars.tr);
    for i in 1..width - 1 {
        grid.set_if_empty(x + i, y, chars.h);
    }

    // Vertical sides
    for i in 1..height - 1 {
        grid.set_if_empty(x, y + i, chars.v);
        grid.set_if_empty(x + width - 1, y + i, chars.v);
    }

    // Bottom row: wavy
    grid.set_if_empty(x, y + height - 1, chars.bl);
    grid.set_if_empty(x + width - 1, y + height - 1, chars.br);
    for i in 1..width - 1 {
        let c = if i % 3 == 1 { '~' } else { chars.h };
        grid.set_if_empty(x + i, y + height - 1, c);
    }

    draw_label(grid, node);
}

/// Draw the label centered in the node (supports multi-line via \n)
fn draw_label(grid: &mut Grid, node: &Node) {
    let lines: Vec<&str> = node.label.split('\n').collect();
    let line_count = lines.len();
    // Vertically center the block of lines within the node
    let block_start_y = node.y + (node.height.saturating_sub(line_count)) / 2;
    for (line_idx, line) in lines.iter().enumerate() {
        let line_w = display_width(line);
        let label_x = node.x + (node.width.saturating_sub(line_w)) / 2;
        let label_y = block_start_y + line_idx;
        draw_text(grid, label_x, label_y, line);
    }
}

/// Draw text at position, advancing x by per-char display width (CJK-aware)
fn draw_text(grid: &mut Grid, x: usize, y: usize, text: &str) {
    let mut dx = 0;
    for c in text.chars() {
        grid.set_if_empty(x + dx, y, c);
        dx += UnicodeWidthChar::width(c).unwrap_or(1);
    }
}
