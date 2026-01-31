//! Pie chart parser and renderer for Mermaid syntax
//!
//! Pie charts are rendered as ASCII bar charts in terminal

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

/// Parse pie chart syntax
pub fn parse_pie_chart(input: &str) -> Result<PieChart, MermaidError> {
    let lines: Vec<&str> = input
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with("%%"))
        .collect();

    if lines.is_empty() {
        return Err(MermaidError::EmptyInput);
    }

    // Validate header
    let first_line = lines[0].to_lowercase();
    if !first_line.starts_with("pie") {
        return Err(MermaidError::ParseError {
            line: 1,
            message: "Expected 'pie' diagram type".to_string(),
            suggestion: Some("Start with 'pie' or 'pie showData'".to_string()),
        });
    }

    let show_data = first_line.contains("showdata");
    let mut title = None;
    let mut slices = Vec::new();

    for line in lines.iter().skip(1) {
        // Parse title
        if line.to_lowercase().starts_with("title") {
            let title_text = line.strip_prefix("title").unwrap_or(line);
            let title_text = title_text.strip_prefix("Title").unwrap_or(title_text);
            title = Some(title_text.trim().to_string());
            continue;
        }

        // Parse slice: "Label" : value
        if let Some((label, value)) = parse_slice(line) {
            slices.push(PieSlice { label, value });
        }
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

/// Parse a slice line: "Label" : value
fn parse_slice(line: &str) -> Option<(String, f64)> {
    // Find the colon separator
    let colon_idx = line.find(':')?;

    let label_part = line[..colon_idx].trim();
    let value_part = line[colon_idx + 1..].trim();

    // Extract label (remove quotes)
    let label = label_part.trim_matches('"').trim_matches('\'').to_string();

    // Parse value
    let value: f64 = value_part.parse().ok()?;

    Some((label, value))
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
}
