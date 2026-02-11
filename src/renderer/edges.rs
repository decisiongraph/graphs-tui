//! Edge drawing and routing functions

use crate::grid::Grid;
use crate::pathfinding::{PathGrid, Pos};
use crate::text::display_width;
use crate::types::{Direction, Edge, EdgeStyle, Node};

use super::charset::CharSet;

/// A label that couldn't be rendered inline on an edge
pub struct DroppedLabel {
    pub marker: String,
    pub label: String,
    pub from: String,
    pub to: String,
}

/// Get line characters for edge style
pub fn get_edge_chars(style: EdgeStyle, chars: &CharSet, ascii: bool) -> (char, char) {
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
pub fn style_has_arrow(style: EdgeStyle) -> bool {
    matches!(
        style,
        EdgeStyle::Arrow | EdgeStyle::DottedArrow | EdgeStyle::ThickArrow
    )
}

/// Draw a path found by A* pathfinding
pub fn draw_astar_path(
    grid: &mut Grid,
    path: &[Pos],
    h_char: char,
    v_char: char,
    arrow_char: char,
    chars: &CharSet,
) {
    if path.is_empty() {
        return;
    }

    let jchars = chars.to_junction_chars();

    for i in 0..path.len() {
        let pos = path[i];

        if i == path.len() - 1 {
            // Last position - draw arrow, check if diagonal
            let final_arrow = if i > 0 {
                let prev = path[i - 1];
                get_arrow_for_direction(prev, pos, arrow_char, chars)
            } else {
                arrow_char
            };
            grid.set_if_empty(pos.x, pos.y, final_arrow);
        } else {
            // Determine direction
            let next = path[i + 1];
            let prev = if i > 0 { Some(path[i - 1]) } else { None };

            let is_horizontal = pos.y == next.y;
            let is_turn = prev.is_some_and(|p| (p.y == pos.y) != is_horizontal);

            if let (true, Some(prev_pos)) = (is_turn, prev) {
                // Draw corner
                let corner = determine_corner(prev_pos, pos, next, chars);
                grid.set_if_empty(pos.x, pos.y, corner);
            } else if is_horizontal {
                grid.set_line_with_merge(pos.x, pos.y, h_char, true, &jchars);
            } else {
                grid.set_line_with_merge(pos.x, pos.y, v_char, false, &jchars);
            }
        }
    }
}

/// Get the appropriate arrow character based on movement direction
pub fn get_arrow_for_direction(from: Pos, to: Pos, default_arrow: char, chars: &CharSet) -> char {
    let dx = to.x as isize - from.x as isize;
    let dy = to.y as isize - from.y as isize;

    match (dx.signum(), dy.signum()) {
        (1, 0) => chars.arr_r,    // right
        (-1, 0) => chars.arr_l,   // left
        (0, 1) => chars.arr_d,    // down
        (0, -1) => chars.arr_u,   // up
        (1, 1) => chars.arr_dr,   // down-right
        (-1, 1) => chars.arr_dl,  // down-left
        (1, -1) => chars.arr_ur,  // up-right
        (-1, -1) => chars.arr_ul, // up-left
        _ => default_arrow,
    }
}

/// Determine the corner character based on path direction
fn determine_corner(prev: Pos, curr: Pos, next: Pos, chars: &CharSet) -> char {
    let from_left = prev.x < curr.x;
    let from_right = prev.x > curr.x;
    let from_above = prev.y < curr.y;
    let from_below = prev.y > curr.y;

    let to_right = next.x > curr.x;
    let to_left = next.x < curr.x;
    let to_below = next.y > curr.y;
    let to_above = next.y < curr.y;

    // Determine corner type
    if (from_left && to_below) || (from_above && to_right) {
        chars.tr // ┐ or coming from left going down, or from above going right
    } else if (from_right && to_below) || (from_above && to_left) {
        chars.tl // ┌
    } else if (from_left && to_above) || (from_below && to_right) {
        chars.br // ┘
    } else if (from_right && to_above) || (from_below && to_left) {
        chars.bl // └
    } else {
        chars.cross // Default to cross if unclear
    }
}

/// Draw an edge between two nodes using A* pathfinding when beneficial
pub fn draw_edge(
    grid: &mut Grid,
    path_grid: &PathGrid,
    from: &Node,
    to: &Node,
    edge: &Edge,
    chars: &CharSet,
    direction: Direction,
    ascii: bool,
    dropped_labels: &mut Vec<DroppedLabel>,
    next_marker: &mut usize,
) {
    let has_arrow = style_has_arrow(edge.style);
    let (h_char, v_char) = get_edge_chars(edge.style, chars, ascii);

    let (start_x, start_y, end_x, end_y, arrow_char) = match direction {
        Direction::LR => (
            from.x + from.width,
            from.y + from.height / 2,
            to.x,
            to.y + to.height / 2,
            if has_arrow { chars.arr_r } else { h_char },
        ),
        Direction::RL => (
            from.x,
            from.y + from.height / 2,
            to.x + to.width,
            to.y + to.height / 2,
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

    // Try A* pathfinding for non-straight edges
    let use_astar = start_x != end_x && start_y != end_y;
    if use_astar {
        if let Some(path) = path_grid.find_path(Pos::new(start_x, start_y), Pos::new(end_x, end_y))
        {
            // Draw the A* path
            draw_astar_path(grid, &path, h_char, v_char, arrow_char, chars);

            // Handle label for A* path
            if let Some(lbl) = &edge.label {
                // Try to place label in the middle of the path
                if path.len() > 2 {
                    let mid_idx = path.len() / 2;
                    let mid_pos = path[mid_idx];
                    // Draw label to the right/below the mid point
                    for (i, c) in lbl.chars().enumerate() {
                        grid.set_if_empty(mid_pos.x + 1 + i, mid_pos.y, c);
                    }
                } else {
                    // Path too short for inline label - drop to legend
                    let marker_text = format!("[{}]", *next_marker);
                    dropped_labels.push(DroppedLabel {
                        marker: marker_text,
                        label: lbl.clone(),
                        from: edge.from.clone(),
                        to: edge.to.clone(),
                    });
                    *next_marker += 1;
                }
            }
            return;
        }
    }

    // Fall back to existing L-shaped routing
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
            &edge.from,
            &edge.to,
            dropped_labels,
            next_marker,
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
            &edge.from,
            &edge.to,
            dropped_labels,
            next_marker,
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
    from_id: &str,
    to_id: &str,
    dropped_labels: &mut Vec<DroppedLabel>,
    next_marker: &mut usize,
) {
    let jchars = chars.to_junction_chars();

    if start_y == end_y {
        // Straight horizontal line
        let (from_x, to_x) = if end_x > start_x {
            (start_x, end_x)
        } else {
            (end_x, start_x)
        };
        for x in from_x..to_x {
            grid.set_line_with_merge(x, start_y, h_char, true, &jchars);
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
            if edge_len >= display_width(lbl) {
                let label_x = from_x + (edge_len - display_width(lbl)) / 2;
                for (i, c) in lbl.chars().enumerate() {
                    grid.set_if_empty(label_x + i, start_y, c);
                }
            } else {
                // Label doesn't fit — try rendering marker, record for legend
                let marker_text = format!("[{}]", *next_marker);
                if edge_len >= marker_text.len() {
                    let marker_x = from_x + (edge_len - marker_text.len()) / 2;
                    for (i, c) in marker_text.chars().enumerate() {
                        grid.set_if_empty(marker_x + i, start_y, c);
                    }
                }
                dropped_labels.push(DroppedLabel {
                    marker: marker_text,
                    label: lbl.to_string(),
                    from: from_id.to_string(),
                    to: to_id.to_string(),
                });
                *next_marker += 1;
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
            grid.set_line_with_merge(x, start_y, h_char, true, &jchars);
        }

        // Turn 1 at (mid_x, start_y)
        let corner1 = if end_y > start_y {
            if is_lr {
                chars.tr
            } else {
                chars.tl
            }
        } else if is_lr {
            chars.br
        } else {
            chars.bl
        };
        grid.set_if_empty(mid_x, start_y, corner1);

        // Vertical from start_y to end_y
        let (from_y, to_y) = if end_y > start_y {
            (start_y + 1, end_y)
        } else {
            (end_y + 1, start_y)
        };
        for y in from_y..to_y {
            grid.set_line_with_merge(mid_x, y, v_char, false, &jchars);
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
            } else {
                // Vertical segment too short for label
                let marker_text = format!("[{}]", *next_marker);
                dropped_labels.push(DroppedLabel {
                    marker: marker_text,
                    label: lbl.to_string(),
                    from: from_id.to_string(),
                    to: to_id.to_string(),
                });
                *next_marker += 1;
            }
        }

        // Turn 2 at (mid_x, end_y)
        let corner2 = if end_y > start_y {
            if is_lr {
                chars.bl
            } else {
                chars.br
            }
        } else if is_lr {
            chars.tl
        } else {
            chars.tr
        };
        grid.set_if_empty(mid_x, end_y, corner2);

        // Horizontal from mid to end
        let (from_x, to_x) = if end_x > mid_x {
            (mid_x + 1, end_x)
        } else {
            (end_x, mid_x)
        };
        for x in from_x..to_x {
            grid.set_line_with_merge(x, end_y, h_char, true, &jchars);
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
    from_id: &str,
    to_id: &str,
    dropped_labels: &mut Vec<DroppedLabel>,
    next_marker: &mut usize,
) {
    let jchars = chars.to_junction_chars();

    if start_x == end_x {
        // Straight vertical line
        let (from_y, to_y) = if end_y > start_y {
            (start_y, end_y)
        } else {
            (end_y, start_y)
        };
        for y in from_y..to_y {
            grid.set_line_with_merge(start_x, y, v_char, false, &jchars);
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
            } else {
                // Edge too short for label
                let marker_text = format!("[{}]", *next_marker);
                dropped_labels.push(DroppedLabel {
                    marker: marker_text,
                    label: lbl.to_string(),
                    from: from_id.to_string(),
                    to: to_id.to_string(),
                });
                *next_marker += 1;
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
            grid.set_line_with_merge(start_x, y, v_char, false, &jchars);
        }

        // Turn 1 at (start_x, mid_y)
        let corner1 = if end_x > start_x {
            if is_tb {
                chars.bl
            } else {
                chars.tl
            }
        } else if is_tb {
            chars.br
        } else {
            chars.tr
        };
        grid.set_if_empty(start_x, mid_y, corner1);

        // Horizontal from start_x to end_x
        let (from_x, to_x) = if end_x > start_x {
            (start_x + 1, end_x)
        } else {
            (end_x + 1, start_x)
        };
        for x in from_x..to_x {
            grid.set_line_with_merge(x, mid_y, h_char, true, &jchars);
        }

        // Draw label — try horizontal segment first, fall back to vertical segment
        if let Some(lbl) = label {
            let horiz_len = to_x.saturating_sub(from_x);
            if horiz_len >= display_width(lbl) {
                let label_x = from_x + (horiz_len - display_width(lbl)) / 2;
                for (i, c) in lbl.chars().enumerate() {
                    grid.set_if_empty(label_x + i, mid_y, c);
                }
            } else {
                // Try placing label alongside the first vertical segment
                let vert_len = mid_y.saturating_sub(start_y);
                if vert_len > 0 {
                    let label_y = start_y + vert_len / 2;
                    for (i, c) in lbl.chars().enumerate() {
                        grid.set_if_empty(start_x + 1 + i, label_y, c);
                    }
                } else {
                    // Label doesn't fit anywhere — drop to legend
                    let marker_text = format!("[{}]", *next_marker);
                    if horiz_len >= marker_text.len() {
                        let marker_x = from_x + (horiz_len - marker_text.len()) / 2;
                        for (i, c) in marker_text.chars().enumerate() {
                            grid.set_if_empty(marker_x + i, mid_y, c);
                        }
                    }
                    dropped_labels.push(DroppedLabel {
                        marker: marker_text,
                        label: lbl.to_string(),
                        from: from_id.to_string(),
                        to: to_id.to_string(),
                    });
                    *next_marker += 1;
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
        } else if is_tb {
            chars.tl
        } else {
            chars.bl
        };
        grid.set_if_empty(end_x, mid_y, corner2);

        // Vertical from mid to end
        let (from_y, to_y) = if end_y > mid_y {
            (mid_y + 1, end_y)
        } else {
            (end_y, mid_y)
        };
        for y in from_y..to_y {
            grid.set_line_with_merge(end_x, y, v_char, false, &jchars);
        }

        // Arrow
        if end_y > mid_y {
            grid.set_if_empty(end_x, end_y - 1, arrow_char);
        } else {
            grid.set_if_empty(end_x, end_y + 1, arrow_char);
        }
    }
}
