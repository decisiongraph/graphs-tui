//! Text display width utilities for proper Unicode handling

use unicode_width::UnicodeWidthStr;

/// Return the display width of a string, accounting for CJK double-width characters.
pub fn display_width(s: &str) -> usize {
    UnicodeWidthStr::width(s)
}
