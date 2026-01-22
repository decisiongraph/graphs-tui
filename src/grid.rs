use std::fmt;

/// 2D character grid for rendering
pub struct Grid {
    cells: Vec<Vec<char>>,
    pub width: usize,
    pub height: usize,
}

impl Grid {
    /// Create a new grid filled with spaces
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            cells: vec![vec![' '; width]; height],
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
}
