use std::fmt;

/// 2D character grid for rendering
pub struct Grid {
    cells: Vec<Vec<char>>,
    /// Cells that are protected from being overwritten by edges
    protected: Vec<Vec<bool>>,
    pub width: usize,
    pub height: usize,
}

impl Grid {
    /// Create a new grid filled with spaces
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            cells: vec![vec![' '; width]; height],
            protected: vec![vec![false; width]; height],
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

impl fmt::Display for Grid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, row) in self.cells.iter().enumerate() {
            let line: String = row.iter().collect();
            write!(f, "{}", line)?;
            if i < self.cells.len() - 1 {
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
        assert_eq!(s, "A  \n  B");
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
}
