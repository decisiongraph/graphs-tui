//! Subgraph rendering functions

use crate::grid::Grid;
use crate::text::display_width;
use crate::types::Subgraph;
use unicode_width::UnicodeWidthChar;

use super::charset::CharSet;

/// Draw a subgraph box
pub fn draw_subgraph(grid: &mut Grid, sg: &Subgraph, chars: &CharSet) {
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
    let label_w = display_width(&sg.label);
    if !sg.label.is_empty() && width > label_w + 2 {
        let label_x = x + (width - label_w) / 2;
        let mut dx = 0;
        for c in sg.label.chars() {
            grid.set(label_x + dx, y, c);
            dx += UnicodeWidthChar::width(c).unwrap_or(1);
        }
    }
}

/// Protect subgraph border cells so nodes/edges can't overwrite them
pub fn protect_subgraph_borders(grid: &mut Grid, sg: &Subgraph) {
    if sg.width == 0 || sg.height == 0 {
        return;
    }

    let x = sg.x;
    let y = sg.y;
    let width = sg.width;
    let height = sg.height;

    // Protect corners
    grid.mark_protected(x, y);
    grid.mark_protected(x + width - 1, y);
    grid.mark_protected(x, y + height - 1);
    grid.mark_protected(x + width - 1, y + height - 1);

    // Protect horizontal lines (top and bottom)
    for i in 1..width - 1 {
        grid.mark_protected(x + i, y);
        grid.mark_protected(x + i, y + height - 1);
    }

    // Protect vertical lines (left and right)
    for i in 1..height - 1 {
        grid.mark_protected(x, y + i);
        grid.mark_protected(x + width - 1, y + i);
    }
}
