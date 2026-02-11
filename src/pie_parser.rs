//! Pie chart parser and renderer for Mermaid syntax
//!
//! Pie charts are rendered as ASCII bar charts in terminal

use winnow::ascii::{digit1, space0, space1};
use winnow::combinator::{alt, delimited, opt, preceded};
use winnow::error::{ErrMode, ParserError};
use winnow::token::{take_until, take_while};
use winnow::ModalResult;
use winnow::Parser;

use crate::error::MermaidError;
use crate::types::RenderOptions;

/// A slice of the pie chart
#[derive(Debug, Clone)]
pub struct PieSlice {
    pub label: String,
    pub value: f64,
}

/// Pie chart data
#[derive(Debug, Clone)]
pub struct PieChart {
    pub title: Option<String>,
    pub slices: Vec<PieSlice>,
    #[allow(dead_code)] // Parsed but not yet used in rendering
    pub show_data: bool,
}

/// Content of a single line (after trimming)
#[derive(Debug)]
enum PieLine {
    Header { show_data: bool },
    Title(String),
    Slice { label: String, value: f64 },
    Comment,
    Empty,
}

/// Parse "pie" keyword, optionally followed by "showData"
fn parse_pie_header(input: &mut &str) -> ModalResult<bool> {
    let _ = winnow::ascii::Caseless("pie").parse_next(input)?;
    let _ = space0.parse_next(input)?;
    let show_data = opt(winnow::ascii::Caseless("showdata"))
        .parse_next(input)?
        .is_some();
    Ok(show_data)
}

/// Parse title line: "title <text>"
fn parse_title_line(input: &mut &str) -> ModalResult<String> {
    let _ = winnow::ascii::Caseless("title").parse_next(input)?;
    let _ = space1.parse_next(input)?;
    let title = take_while(1.., |c| c != '\n').parse_next(input)?;
    Ok(title.trim().to_string())
}

/// Parse a quoted string: "..." or '...'
fn parse_quoted_string(input: &mut &str) -> ModalResult<String> {
    alt((
        delimited('"', take_until(0.., "\""), '"'),
        delimited('\'', take_until(0.., "'"), '\''),
    ))
    .map(|s: &str| s.to_string())
    .parse_next(input)
}

/// Parse a number (integer or float)
fn parse_number(input: &mut &str) -> ModalResult<f64> {
    let int_part = digit1.parse_next(input)?;
    let frac_part = opt(preceded('.', digit1)).parse_next(input)?;

    let num_str = if let Some(frac) = frac_part {
        format!("{}.{}", int_part, frac)
    } else {
        int_part.to_string()
    };

    num_str.parse().map_err(|_| ErrMode::from_input(input))
}

/// Parse a slice line: "Label" : value
fn parse_slice_line(input: &mut &str) -> ModalResult<(String, f64)> {
    let _ = space0.parse_next(input)?;
    let label = parse_quoted_string.parse_next(input)?;
    let _ = space0.parse_next(input)?;
    let _ = ':'.parse_next(input)?;
    let _ = space0.parse_next(input)?;
    let value = parse_number.parse_next(input)?;
    Ok((label, value))
}

/// Parse a single line and classify it
fn parse_line(line: &str) -> PieLine {
    let trimmed = line.trim();

    // Empty line
    if trimmed.is_empty() {
        return PieLine::Empty;
    }

    // Comment
    if trimmed.starts_with("%%") {
        return PieLine::Comment;
    }

    // Try pie header
    if let Ok(show_data) = parse_pie_header.parse(trimmed) {
        return PieLine::Header { show_data };
    }

    // Try title
    if let Ok(title) = parse_title_line.parse(trimmed) {
        return PieLine::Title(title);
    }

    // Try slice
    if let Ok((label, value)) = parse_slice_line.parse(trimmed) {
        return PieLine::Slice { label, value };
    }

    // Unknown line - treat as empty
    PieLine::Empty
}

/// Parse pie chart syntax
pub fn parse_pie_chart(input: &str) -> Result<PieChart, MermaidError> {
    let lines: Vec<&str> = input.lines().collect();

    if lines.is_empty() || lines.iter().all(|l| l.trim().is_empty()) {
        return Err(MermaidError::EmptyInput);
    }

    let mut show_data = false;
    let mut title = None;
    let mut slices = Vec::new();
    let mut found_header = false;

    for line in lines.iter() {
        match parse_line(line) {
            PieLine::Header { show_data: sd } => {
                if !found_header {
                    found_header = true;
                    show_data = sd;
                }
            }
            PieLine::Title(t) => {
                title = Some(t);
            }
            PieLine::Slice { label, value } => {
                slices.push(PieSlice { label, value });
            }
            PieLine::Comment | PieLine::Empty => {}
        }
    }

    if !found_header {
        return Err(MermaidError::ParseError {
            line: 1,
            message: "Expected 'pie' diagram type".to_string(),
            suggestion: Some("Start with 'pie' or 'pie showData'".to_string()),
        });
    }

    if slices.is_empty() {
        return Err(MermaidError::ParseError {
            line: 1,
            message: "No pie chart data found".to_string(),
            suggestion: Some("Add slices like '\"Chrome\" : 65'".to_string()),
        });
    }

    Ok(PieChart {
        title,
        slices,
        show_data,
    })
}

/// Render pie chart to ASCII representation
pub fn render_pie_chart(chart: &PieChart, _options: &RenderOptions) -> String {
    let mut output = String::new();

    // Calculate total for percentages
    let total: f64 = chart.slices.iter().map(|s| s.value).sum();
    if total == 0.0 {
        return "No data".to_string();
    }

    // Title
    if let Some(ref title) = chart.title {
        output.push_str(&format!("  {}\n", title));
        output.push_str(&format!("  {}\n\n", "─".repeat(title.len())));
    }

    // Find max label width for alignment
    let max_label_width = chart
        .slices
        .iter()
        .map(|s| s.label.len())
        .max()
        .unwrap_or(10);
    let bar_width = 30;

    // Render each slice as a horizontal bar
    for slice in &chart.slices {
        let percentage = (slice.value / total) * 100.0;
        let bar_length = ((percentage / 100.0) * bar_width as f64).round() as usize;

        // Bar character based on percentage
        let bar_char = if percentage >= 50.0 {
            '█'
        } else if percentage >= 25.0 {
            '▓'
        } else if percentage >= 10.0 {
            '▒'
        } else {
            '░'
        };

        let bar: String = std::iter::repeat_n(bar_char, bar_length).collect();
        let padding: String = " ".repeat(bar_width - bar_length);

        // Format: Label  |████████████| value (percentage%)
        output.push_str(&format!(
            "  {:width$}  │{}{}│ {:.0} ({:.1}%)\n",
            slice.label,
            bar,
            padding,
            slice.value,
            percentage,
            width = max_label_width
        ));
    }

    // Total
    output.push_str(&format!(
        "\n  {:width$}  Total: {:.0}\n",
        "",
        total,
        width = max_label_width
    ));

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pie_simple() {
        let input = r#"pie
    "Chrome" : 65
    "Firefox" : 15
"#;
        let chart = parse_pie_chart(input).unwrap();
        assert_eq!(chart.slices.len(), 2);
        assert_eq!(chart.slices[0].label, "Chrome");
        assert_eq!(chart.slices[0].value, 65.0);
    }

    #[test]
    fn test_parse_pie_with_title() {
        let input = r#"pie
    title Browser Share
    "Chrome" : 65
"#;
        let chart = parse_pie_chart(input).unwrap();
        assert_eq!(chart.title, Some("Browser Share".to_string()));
    }

    #[test]
    fn test_parse_pie_show_data() {
        let input = r#"pie showData
    "Yes" : 70
"#;
        let chart = parse_pie_chart(input).unwrap();
        assert!(chart.show_data);
    }

    #[test]
    fn test_render_pie() {
        let chart = PieChart {
            title: Some("Test".to_string()),
            slices: vec![
                PieSlice {
                    label: "A".to_string(),
                    value: 60.0,
                },
                PieSlice {
                    label: "B".to_string(),
                    value: 40.0,
                },
            ],
            show_data: false,
        };
        let output = render_pie_chart(&chart, &RenderOptions::default());
        assert!(output.contains("Test"));
        assert!(output.contains("A"));
        assert!(output.contains("B"));
        assert!(output.contains("60"));
        assert!(output.contains("40"));
    }

    #[test]
    fn test_parse_quoted_string() {
        assert_eq!(
            parse_quoted_string.parse("\"Hello\"").unwrap(),
            "Hello".to_string()
        );
        assert_eq!(
            parse_quoted_string.parse("'World'").unwrap(),
            "World".to_string()
        );
    }

    #[test]
    fn test_parse_number() {
        assert_eq!(parse_number.parse("42").unwrap(), 42.0);
        assert_eq!(parse_number.parse("3.14").unwrap(), 3.14);
    }

    #[test]
    fn test_parse_slice_line() {
        let result = parse_slice_line.parse("\"Chrome\" : 65").unwrap();
        assert_eq!(result.0, "Chrome");
        assert_eq!(result.1, 65.0);
    }
}
