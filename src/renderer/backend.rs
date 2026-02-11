//! Render backend trait for abstracting rendering operations

use crate::grid::JunctionChars;

/// Trait for render backends that can draw characters to a 2D surface
#[allow(dead_code)]
pub trait RenderBackend {
    /// Set a character at given position (unconditional, may overwrite)
    fn set(&mut self, x: usize, y: usize, c: char);

    /// Set a character only if the cell is not protected
    /// Returns true if the character was set
    fn set_if_empty(&mut self, x: usize, y: usize, c: char) -> bool;

    /// Mark a cell as protected (won't be overwritten by subsequent set_if_empty calls)
    fn mark_protected(&mut self, x: usize, y: usize);

    /// Set a line character with junction merging
    /// If the cell already has a line in a different direction, merge into a junction
    /// `is_horizontal` indicates if this is a horizontal line
    fn set_line_with_merge(
        &mut self,
        x: usize,
        y: usize,
        c: char,
        is_horizontal: bool,
        chars: &JunctionChars,
    ) -> bool;

    /// Get the dimensions of the rendering surface
    fn dimensions(&self) -> (usize, usize);

    /// Get character at given position
    fn get(&self, x: usize, y: usize) -> Option<char>;
}

// Grid implements RenderBackend via its existing methods
// The trait is implemented in grid.rs
