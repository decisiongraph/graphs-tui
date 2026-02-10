use std::fmt;

/// Line direction flags for junction merging
#[derive(Clone, Copy, Default)]
pub struct LineFlags {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
}

/// 2D character grid for rendering
pub struct Grid {
    cells: Vec<Vec<char>>,
    /// Cells that are protected from being overwritten by edges
    protected: Vec<Vec<bool>>,
    /// Track line directions at each cell for junction merging
    line_flags: Vec<Vec<LineFlags>>,
    pub width: usize,
    pub height: usize,
}

impl Grid {
    /// Create a new grid filled with spaces
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            cells: vec![vec![' '; width]; height],
            protected: vec![vec![false; width]; height],
            line_flags: vec![vec![LineFlags::default(); width]; height],
            width,
            height,
        }
    }

    /// Set a character at given position (bounds-checked)
    pub fn set(&mut self, x: usize, y: usize, c: char) {
        if x < self.width && y < self.height {
            self.cells[y][x] = c;
        }
    }

    /// Set a character and mark it as protected (won't be overwritten by edges)
    #[allow(dead_code)]
    pub fn set_protected(&mut self, x: usize, y: usize, c: char) {
        if x < self.width && y < self.height {
            self.cells[y][x] = c;
            self.protected[y][x] = true;
        }
    }

    /// Mark a cell as protected without changing its content
    pub fn mark_protected(&mut self, x: usize, y: usize) {
        if x < self.width && y < self.height {
            self.protected[y][x] = true;
        }
    }

    /// Set a character only if the cell is not protected
    /// Returns true if the character was set
    pub fn set_if_empty(&mut self, x: usize, y: usize, c: char) -> bool {
        if x < self.width && y < self.height && !self.protected[y][x] {
            self.cells[y][x] = c;
            return true;
        }
        false
    }

    /// Set a line character with junction merging.
    /// If the cell already has a line in a different direction, merge into a junction.
    /// `is_horizontal` indicates if this is a horizontal line.
    /// Returns true if the character was set.
    pub fn set_line_with_merge(
        &mut self,
        x: usize,
        y: usize,
        c: char,
        is_horizontal: bool,
        chars: &JunctionChars,
    ) -> bool {
        if x >= self.width || y >= self.height || self.protected[y][x] {
            return false;
        }

        // Update line flags
        if is_horizontal {
            self.line_flags[y][x].left = true;
            self.line_flags[y][x].right = true;
        } else {
            self.line_flags[y][x].up = true;
            self.line_flags[y][x].down = true;
        }

        // Compute merged character based on flags
        let flags = &self.line_flags[y][x];
        let has_h = flags.left || flags.right;
        let has_v = flags.up || flags.down;

        self.cells[y][x] = if has_h && has_v {
            // Both horizontal and vertical - use cross
            chars.cross
        } else {
            c
        };
        true
    }

    /// Check if a cell is protected
    #[allow(dead_code)]
    pub fn is_protected(&self, x: usize, y: usize) -> bool {
        if x < self.width && y < self.height {
            self.protected[y][x]
        } else {
            true // Out of bounds treated as protected
        }
    }

    /// Get character at given position
    #[allow(dead_code)]
    pub fn get(&self, x: usize, y: usize) -> Option<char> {
        if x < self.width && y < self.height {
            Some(self.cells[y][x])
        } else {
            None
        }
    }

}

/// Junction characters needed for line merging
#[allow(dead_code)]
pub struct JunctionChars {
    pub cross: char,  // ┼
    pub t_up: char,   // ┴ (for future T-junction support)
    pub t_down: char, // ┬ (for future T-junction support)
    pub ml: char,     // ├ (for future T-junction support)
    pub mr: char,     // ┤ (for future T-junction support)
}

impl fmt::Display for Grid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Find the last row that has non-space content
        let last_non_empty = self
            .cells
            .iter()
            .rposition(|row| row.iter().any(|&c| c != ' '))
            .unwrap_or(0);

        for (i, row) in self.cells[..=last_non_empty].iter().enumerate() {
            let line: String = row.iter().collect();
            let trimmed = line.trim_end();
            write!(f, "{}", trimmed)?;
            if i < last_non_empty {
                writeln!(f)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_creation() {
        let grid = Grid::new(5, 3);
        assert_eq!(grid.width, 5);
        assert_eq!(grid.height, 3);
    }

    #[test]
    fn test_grid_set_get() {
        let mut grid = Grid::new(5, 3);
        grid.set(2, 1, 'X');
        assert_eq!(grid.get(2, 1), Some('X'));
        assert_eq!(grid.get(0, 0), Some(' '));
    }

    #[test]
    fn test_grid_bounds_check() {
        let mut grid = Grid::new(5, 3);
        grid.set(10, 10, 'X'); // Should not panic
        assert_eq!(grid.get(10, 10), None);
    }

    #[test]
    fn test_grid_display() {
        let mut grid = Grid::new(3, 2);
        grid.set(0, 0, 'A');
        grid.set(2, 1, 'B');
        let s = grid.to_string();
        assert_eq!(s, "A\n  B");
    }

    #[test]
    fn test_grid_protected() {
        let mut grid = Grid::new(5, 3);
        grid.set_protected(2, 1, 'N'); // Protected node cell
        assert!(grid.is_protected(2, 1));

        // Try to overwrite with edge - should fail
        let written = grid.set_if_empty(2, 1, '│');
        assert!(!written);
        assert_eq!(grid.get(2, 1), Some('N')); // Original char preserved
    }

    #[test]
    fn test_grid_set_if_empty() {
        let mut grid = Grid::new(5, 3);

        // Non-protected cell - should work
        let written = grid.set_if_empty(1, 1, '─');
        assert!(written);
        assert_eq!(grid.get(1, 1), Some('─'));
    }

    #[test]
    fn test_junction_merging() {
        let mut grid = Grid::new(5, 5);
        let jchars = JunctionChars {
            cross: '┼',
            t_up: '┴',
            t_down: '┬',
            ml: '├',
            mr: '┤',
        };

        // Draw horizontal line through center
        grid.set_line_with_merge(1, 2, '─', true, &jchars);
        grid.set_line_with_merge(2, 2, '─', true, &jchars);
        grid.set_line_with_merge(3, 2, '─', true, &jchars);

        // Draw vertical line through center - should create cross at (2,2)
        grid.set_line_with_merge(2, 1, '│', false, &jchars);
        grid.set_line_with_merge(2, 2, '│', false, &jchars);
        grid.set_line_with_merge(2, 3, '│', false, &jchars);

        // The cell at (2,2) should be a cross since both horizontal and vertical pass through
        assert_eq!(grid.get(2, 2), Some('┼'));
    }
}
