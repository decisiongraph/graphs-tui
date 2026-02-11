//! Character sets for rendering diagrams

use crate::grid::JunctionChars;

/// Unicode box-drawing characters
pub struct CharSet {
    pub tl: char,    // top-left corner
    pub tr: char,    // top-right corner
    pub bl: char,    // bottom-left corner
    pub br: char,    // bottom-right corner
    pub h: char,     // horizontal line
    pub v: char,     // vertical line
    pub arr_r: char, // arrow right
    pub arr_l: char, // arrow left
    pub arr_d: char, // arrow down
    pub arr_u: char, // arrow up
    // Diagonal arrows for non-orthogonal edges
    pub arr_dr: char, // arrow down-right (◢)
    pub arr_dl: char, // arrow down-left (◣)
    pub arr_ur: char, // arrow up-right (◥)
    pub arr_ul: char, // arrow up-left (◤)
    // Rounded corners
    pub rtl: char,
    pub rtr: char,
    pub rbl: char,
    pub rbr: char,
    // T-junctions (for cylinder separators)
    pub ml: char, // middle-left (├)
    pub mr: char, // middle-right (┤)
    // Double lines for subgraphs
    pub dh: char,
    pub dv: char,
    pub dtl: char,
    pub dtr: char,
    pub dbl: char,
    pub dbr: char,
    // Junction characters for overlapping lines
    pub cross: char,  // cross (┼)
    pub t_up: char,   // T pointing up (┴)
    pub t_down: char, // T pointing down (┬)
}

pub const UNICODE_CHARS: CharSet = CharSet {
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
    arr_dr: '◢',
    arr_dl: '◣',
    arr_ur: '◥',
    arr_ul: '◤',
    rtl: '╭',
    rtr: '╮',
    rbl: '╰',
    rbr: '╯',
    ml: '├',
    mr: '┤',
    dh: '═',
    dv: '║',
    dtl: '╔',
    dtr: '╗',
    dbl: '╚',
    dbr: '╝',
    cross: '┼',
    t_up: '┴',
    t_down: '┬',
};

pub const ASCII_CHARS: CharSet = CharSet {
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
    arr_dr: '\\',
    arr_dl: '/',
    arr_ur: '/',
    arr_ul: '\\',
    rtl: '+',
    rtr: '+',
    rbl: '+',
    rbr: '+',
    ml: '+',
    mr: '+',
    dh: '=',
    dv: '#',
    dtl: '#',
    dtr: '#',
    dbl: '#',
    dbr: '#',
    cross: '+',
    t_up: '+',
    t_down: '+',
};

impl CharSet {
    /// Convert to JunctionChars for grid line merging
    pub fn to_junction_chars(&self) -> JunctionChars {
        JunctionChars {
            cross: self.cross,
            t_up: self.t_up,
            t_down: self.t_down,
            ml: self.ml,
            mr: self.mr,
        }
    }
}
