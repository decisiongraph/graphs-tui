//! A* pathfinding for edge routing around obstacles
//!
//! Finds shortest paths between nodes while avoiding obstacles (other nodes).

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};

/// A position in the grid
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Pos {
    pub x: usize,
    pub y: usize,
}

impl Pos {
    pub fn new(x: usize, y: usize) -> Self {
        Self { x, y }
    }
}

/// A node in the A* search priority queue
#[derive(Clone, Copy, Eq, PartialEq)]
struct AStarNode {
    pos: Pos,
    f_score: usize, // g_score + heuristic
}

impl Ord for AStarNode {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering for min-heap (lower f_score = higher priority)
        other.f_score.cmp(&self.f_score)
    }
}

impl PartialOrd for AStarNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Pathfinding grid with obstacles
pub struct PathGrid {
    width: usize,
    height: usize,
    /// Cells that are blocked (contain nodes/obstacles)
    blocked: HashSet<Pos>,
}

impl PathGrid {
    /// Create a new pathfinding grid
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            blocked: HashSet::new(),
        }
    }

    /// Mark a rectangular region as blocked (e.g., a node)
    pub fn block_rect(&mut self, x: usize, y: usize, width: usize, height: usize) {
        for dy in 0..height {
            for dx in 0..width {
                self.blocked.insert(Pos::new(x + dx, y + dy));
            }
        }
    }

    /// Unblock specific positions (for edge start/end points)
    pub fn unblock(&mut self, pos: Pos) {
        self.blocked.remove(&pos);
    }

    /// Check if a position is valid and not blocked
    fn is_valid(&self, pos: Pos) -> bool {
        pos.x < self.width && pos.y < self.height && !self.blocked.contains(&pos)
    }

    /// Get valid neighbors (4-directional movement)
    fn neighbors(&self, pos: Pos) -> Vec<Pos> {
        let mut result = Vec::new();

        // Right
        if pos.x + 1 < self.width {
            let p = Pos::new(pos.x + 1, pos.y);
            if self.is_valid(p) {
                result.push(p);
            }
        }
        // Left
        if pos.x > 0 {
            let p = Pos::new(pos.x - 1, pos.y);
            if self.is_valid(p) {
                result.push(p);
            }
        }
        // Down
        if pos.y + 1 < self.height {
            let p = Pos::new(pos.x, pos.y + 1);
            if self.is_valid(p) {
                result.push(p);
            }
        }
        // Up
        if pos.y > 0 {
            let p = Pos::new(pos.x, pos.y - 1);
            if self.is_valid(p) {
                result.push(p);
            }
        }

        result
    }

    /// Manhattan distance heuristic with corner penalty
    fn heuristic(from: Pos, to: Pos) -> usize {
        let dx = if from.x > to.x {
            from.x - to.x
        } else {
            to.x - from.x
        };
        let dy = if from.y > to.y {
            from.y - to.y
        } else {
            to.y - from.y
        };
        dx + dy
    }

    /// Find shortest path from start to goal using A*
    /// Returns None if no path exists
    pub fn find_path(&self, start: Pos, goal: Pos) -> Option<Vec<Pos>> {
        if !self.is_valid(start) || !self.is_valid(goal) {
            return None;
        }

        let mut open_set = BinaryHeap::new();
        let mut came_from: HashMap<Pos, Pos> = HashMap::new();
        let mut g_score: HashMap<Pos, usize> = HashMap::new();

        g_score.insert(start, 0);
        open_set.push(AStarNode {
            pos: start,
            f_score: Self::heuristic(start, goal),
        });

        while let Some(current) = open_set.pop() {
            if current.pos == goal {
                // Reconstruct path
                let mut path = vec![current.pos];
                let mut pos = current.pos;
                while let Some(&prev) = came_from.get(&pos) {
                    path.push(prev);
                    pos = prev;
                }
                path.reverse();
                return Some(path);
            }

            let current_g = *g_score.get(&current.pos).unwrap_or(&usize::MAX);

            for neighbor in self.neighbors(current.pos) {
                let tentative_g = current_g + 1;

                if tentative_g < *g_score.get(&neighbor).unwrap_or(&usize::MAX) {
                    came_from.insert(neighbor, current.pos);
                    g_score.insert(neighbor, tentative_g);
                    let f = tentative_g + Self::heuristic(neighbor, goal);
                    open_set.push(AStarNode {
                        pos: neighbor,
                        f_score: f,
                    });
                }
            }
        }

        None // No path found
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_path() {
        let grid = PathGrid::new(10, 10);
        let path = grid.find_path(Pos::new(0, 0), Pos::new(5, 5));
        assert!(path.is_some());
        let path = path.unwrap();
        assert_eq!(path.first(), Some(&Pos::new(0, 0)));
        assert_eq!(path.last(), Some(&Pos::new(5, 5)));
        // Manhattan distance should be 10
        assert_eq!(path.len(), 11); // 10 steps + start
    }

    #[test]
    fn test_path_around_obstacle() {
        let mut grid = PathGrid::new(10, 10);
        // Block a vertical wall from (5, 0) to (5, 7)
        for y in 0..8 {
            grid.block_rect(5, y, 1, 1);
        }

        let path = grid.find_path(Pos::new(3, 5), Pos::new(7, 5));
        assert!(path.is_some());
        let path = path.unwrap();
        // Path should go around the wall (through y=8)
        assert!(path.iter().all(|p| p.x != 5 || p.y >= 8));
    }

    #[test]
    fn test_no_path() {
        let mut grid = PathGrid::new(10, 10);
        // Block entire column
        for y in 0..10 {
            grid.block_rect(5, y, 1, 1);
        }

        let path = grid.find_path(Pos::new(3, 5), Pos::new(7, 5));
        assert!(path.is_none());
    }
}
