//! Sequence diagram parser and renderer for Mermaid syntax
//!
//! Supports basic mermaid sequence diagram syntax

use crate::error::MermaidError;
use crate::types::RenderOptions;

/// A participant in the sequence diagram
#[derive(Debug, Clone)]
pub struct Participant {
    pub id: String,
    pub label: String,
}

/// Message arrow style
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrowStyle {
    /// Solid arrow ->>
    Solid,
    /// Dotted arrow -->>
    Dotted,
    /// Solid line ->
    SolidLine,
    /// Dotted line -->
    DottedLine,
    /// Async arrow -)
    Async,
}

/// A message between participants
#[derive(Debug, Clone)]
pub struct Message {
    pub from: String,
    pub to: String,
    pub label: String,
    pub style: ArrowStyle,
}

/// Sequence diagram data
#[derive(Debug, Clone)]
pub struct SequenceDiagram {
    pub title: Option<String>,
    pub participants: Vec<Participant>,
    pub messages: Vec<Message>,
}

/// Parse sequence diagram syntax
pub fn parse_sequence_diagram(input: &str) -> Result<SequenceDiagram, MermaidError> {
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
    if !first_line.starts_with("sequencediagram") {
        return Err(MermaidError::ParseError {
            line: 1,
            message: "Expected 'sequenceDiagram'".to_string(),
            suggestion: Some("Start with 'sequenceDiagram'".to_string()),
        });
    }

    let mut diagram = SequenceDiagram {
        title: None,
        participants: Vec::new(),
        messages: Vec::new(),
    };

    let mut seen_participants: std::collections::HashSet<String> = std::collections::HashSet::new();

    for line in lines.iter().skip(1) {
        // Parse title
        if line.to_lowercase().starts_with("title") {
            let title_text = line
                .strip_prefix("title")
                .or_else(|| line.strip_prefix("Title"))
                .unwrap_or(line);
            diagram.title = Some(title_text.trim().to_string());
            continue;
        }

        // Parse participant declaration
        if line.to_lowercase().starts_with("participant") {
            if let Some(p) = parse_participant(line) {
                if !seen_participants.contains(&p.id) {
                    seen_participants.insert(p.id.clone());
                    diagram.participants.push(p);
                }
            }
            continue;
        }

        // Parse actor declaration (alias for participant)
        if line.to_lowercase().starts_with("actor") {
            if let Some(p) = parse_actor(line) {
                if !seen_participants.contains(&p.id) {
                    seen_participants.insert(p.id.clone());
                    diagram.participants.push(p);
                }
            }
            continue;
        }

        // Parse message
        if let Some(msg) = parse_message(line) {
            // Auto-add participants if not declared
            if !seen_participants.contains(&msg.from) {
                seen_participants.insert(msg.from.clone());
                diagram.participants.push(Participant {
                    id: msg.from.clone(),
                    label: msg.from.clone(),
                });
            }
            if !seen_participants.contains(&msg.to) {
                seen_participants.insert(msg.to.clone());
                diagram.participants.push(Participant {
                    id: msg.to.clone(),
                    label: msg.to.clone(),
                });
            }
            diagram.messages.push(msg);
        }
    }

    if diagram.participants.is_empty() && diagram.messages.is_empty() {
        return Err(MermaidError::ParseError {
            line: 1,
            message: "No sequence diagram content found".to_string(),
            suggestion: Some("Add messages like 'Alice->>Bob: Hello'".to_string()),
        });
    }

    Ok(diagram)
}

/// Parse participant declaration: participant Alice or participant A as Alice
fn parse_participant(line: &str) -> Option<Participant> {
    let rest = line
        .strip_prefix("participant")
        .or_else(|| line.strip_prefix("Participant"))?
        .trim();

    if rest.contains(" as ") {
        let parts: Vec<&str> = rest.splitn(2, " as ").collect();
        if parts.len() == 2 {
            return Some(Participant {
                id: parts[0].trim().to_string(),
                label: parts[1].trim().to_string(),
            });
        }
    }

    Some(Participant {
        id: rest.to_string(),
        label: rest.to_string(),
    })
}

/// Parse actor declaration: actor Alice or actor A as Alice
fn parse_actor(line: &str) -> Option<Participant> {
    let rest = line
        .strip_prefix("actor")
        .or_else(|| line.strip_prefix("Actor"))?
        .trim();

    if rest.contains(" as ") {
        let parts: Vec<&str> = rest.splitn(2, " as ").collect();
        if parts.len() == 2 {
            return Some(Participant {
                id: parts[0].trim().to_string(),
                label: parts[1].trim().to_string(),
            });
        }
    }

    Some(Participant {
        id: rest.to_string(),
        label: rest.to_string(),
    })
}

/// Parse message: From->>To: Label
fn parse_message(line: &str) -> Option<Message> {
    // Order matters - check longer patterns first
    let patterns = [
        ("-->>", ArrowStyle::Dotted),
        ("->>", ArrowStyle::Solid),
        ("-->", ArrowStyle::DottedLine),
        ("->", ArrowStyle::SolidLine),
        ("-)", ArrowStyle::Async),
    ];

    for (pattern, style) in patterns {
        if let Some(idx) = line.find(pattern) {
            let from = line[..idx].trim().to_string();
            let rest = line[idx + pattern.len()..].trim();

            // Parse label after colon
            let (to, label) = if let Some(colon_idx) = rest.find(':') {
                let to = rest[..colon_idx].trim().to_string();
                let label = rest[colon_idx + 1..].trim().to_string();
                (to, label)
            } else {
                (rest.to_string(), String::new())
            };

            if !from.is_empty() && !to.is_empty() {
                return Some(Message {
                    from,
                    to,
                    label,
                    style,
                });
            }
        }
    }

    None
}

/// Render sequence diagram to ASCII representation
#[allow(clippy::needless_range_loop)]
pub fn render_sequence_diagram(diagram: &SequenceDiagram, options: &RenderOptions) -> String {
    let mut output = String::new();

    if diagram.participants.is_empty() {
        return "No participants".to_string();
    }

    // Character set
    let (box_h, box_v, box_tl, box_tr, box_bl, box_br) = if options.ascii {
        ('-', '|', '+', '+', '+', '+')
    } else {
        ('─', '│', '┌', '┐', '└', '┘')
    };

    let arrow_r = if options.ascii { '>' } else { '▶' };
    let arrow_l = if options.ascii { '<' } else { '◀' };

    // Calculate participant column widths
    let min_col_width = 12;
    let col_widths: Vec<usize> = diagram
        .participants
        .iter()
        .map(|p| (p.label.len() + 4).max(min_col_width))
        .collect();

    // Calculate participant x positions (center of each column)
    let mut positions: Vec<usize> = Vec::new();
    let mut x = 0;
    for width in &col_widths {
        positions.push(x + width / 2);
        x += width;
    }
    let total_width = x;

    // Title
    if let Some(ref title) = diagram.title {
        let padding = (total_width.saturating_sub(title.len())) / 2;
        output.push_str(&" ".repeat(padding));
        output.push_str(title);
        output.push('\n');
        output.push_str(&" ".repeat(padding));
        output.push_str(&"─".repeat(title.len()));
        output.push_str("\n\n");
    }

    // Draw participant boxes at top
    // Box top line
    let mut line = vec![' '; total_width];
    for (i, p) in diagram.participants.iter().enumerate() {
        let center = positions[i];
        let box_width = p.label.len() + 2;
        let start = center.saturating_sub(box_width / 2);
        let end = start + box_width;

        if start < total_width {
            line[start] = box_tl;
        }
        for j in (start + 1)..end.min(total_width).saturating_sub(1) {
            line[j] = box_h;
        }
        if end > 0 && end - 1 < total_width {
            line[end - 1] = box_tr;
        }
    }
    output.push_str(&line.iter().collect::<String>());
    output.push('\n');

    // Box middle line (label)
    let mut line = vec![' '; total_width];
    for (i, p) in diagram.participants.iter().enumerate() {
        let center = positions[i];
        let box_width = p.label.len() + 2;
        let start = center.saturating_sub(box_width / 2);
        let end = start + box_width;

        if start < total_width {
            line[start] = box_v;
        }
        // Center label
        let label_start = start + 1;
        for (j, c) in p.label.chars().enumerate() {
            if label_start + j < total_width {
                line[label_start + j] = c;
            }
        }
        if end > 0 && end - 1 < total_width {
            line[end - 1] = box_v;
        }
    }
    output.push_str(&line.iter().collect::<String>());
    output.push('\n');

    // Box bottom line
    let mut line = vec![' '; total_width];
    for (i, p) in diagram.participants.iter().enumerate() {
        let center = positions[i];
        let box_width = p.label.len() + 2;
        let start = center.saturating_sub(box_width / 2);
        let end = start + box_width;

        if start < total_width {
            line[start] = box_bl;
        }
        for j in (start + 1)..end.min(total_width).saturating_sub(1) {
            line[j] = box_h;
        }
        if end > 0 && end - 1 < total_width {
            line[end - 1] = box_br;
        }
    }
    output.push_str(&line.iter().collect::<String>());
    output.push('\n');

    // Draw vertical lines (lifelines) and messages
    for msg in &diagram.messages {
        // Find participant indices
        let from_idx = diagram
            .participants
            .iter()
            .position(|p| p.id == msg.from || p.label == msg.from);
        let to_idx = diagram
            .participants
            .iter()
            .position(|p| p.id == msg.to || p.label == msg.to);

        if let (Some(from_i), Some(to_i)) = (from_idx, to_idx) {
            let from_x = positions[from_i];
            let to_x = positions[to_i];

            // Draw lifeline row with vertical lines at participant positions
            let mut line = vec![' '; total_width];
            for &pos in &positions {
                if pos < total_width {
                    line[pos] = if options.ascii { '|' } else { '│' };
                }
            }
            output.push_str(&line.iter().collect::<String>());
            output.push('\n');

            // Draw message arrow
            let mut line = vec![' '; total_width];
            for &pos in &positions {
                if pos < total_width {
                    line[pos] = if options.ascii { '|' } else { '│' };
                }
            }

            let (start_x, end_x, going_right) = if from_x < to_x {
                (from_x, to_x, true)
            } else {
                (to_x, from_x, false)
            };

            // Draw arrow line
            let arrow_char = match msg.style {
                ArrowStyle::Dotted | ArrowStyle::DottedLine => {
                    if options.ascii {
                        '-'
                    } else {
                        '·'
                    }
                }
                _ => {
                    if options.ascii {
                        '-'
                    } else {
                        '─'
                    }
                }
            };

            for x in (start_x + 1)..end_x {
                if x < total_width {
                    line[x] = arrow_char;
                }
            }

            // Draw arrow head
            let has_arrow = matches!(
                msg.style,
                ArrowStyle::Solid | ArrowStyle::Dotted | ArrowStyle::Async
            );
            if has_arrow {
                if going_right && end_x > 0 && end_x - 1 < total_width {
                    line[end_x - 1] = arrow_r;
                } else if !going_right && start_x + 1 < total_width {
                    line[start_x + 1] = arrow_l;
                }
            }

            output.push_str(&line.iter().collect::<String>());

            // Add label
            if !msg.label.is_empty() {
                output.push_str("  ");
                output.push_str(&msg.label);
            }
            output.push('\n');
        }
    }

    // Final lifeline row
    let mut line = vec![' '; total_width];
    for &pos in &positions {
        if pos < total_width {
            line[pos] = if options.ascii { '|' } else { '│' };
        }
    }
    output.push_str(&line.iter().collect::<String>());
    output.push('\n');

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_sequence() {
        let input = r#"sequenceDiagram
    Alice->>Bob: Hello
"#;
        let diagram = parse_sequence_diagram(input).unwrap();
        assert_eq!(diagram.participants.len(), 2);
        assert_eq!(diagram.messages.len(), 1);
        assert_eq!(diagram.messages[0].from, "Alice");
        assert_eq!(diagram.messages[0].to, "Bob");
        assert_eq!(diagram.messages[0].label, "Hello");
    }

    #[test]
    fn test_parse_participant_declaration() {
        let input = r#"sequenceDiagram
    participant A as Alice
    participant B as Bob
    A->>B: Hi
"#;
        let diagram = parse_sequence_diagram(input).unwrap();
        assert_eq!(diagram.participants.len(), 2);
        assert_eq!(diagram.participants[0].id, "A");
        assert_eq!(diagram.participants[0].label, "Alice");
    }

    #[test]
    fn test_parse_arrow_styles() {
        let input = r#"sequenceDiagram
    A->>B: Solid
    A-->>B: Dotted
    A->B: Line
    A-->B: DottedLine
"#;
        let diagram = parse_sequence_diagram(input).unwrap();
        assert_eq!(diagram.messages.len(), 4);
        assert_eq!(diagram.messages[0].style, ArrowStyle::Solid);
        assert_eq!(diagram.messages[1].style, ArrowStyle::Dotted);
        assert_eq!(diagram.messages[2].style, ArrowStyle::SolidLine);
        assert_eq!(diagram.messages[3].style, ArrowStyle::DottedLine);
    }

    #[test]
    fn test_render_sequence() {
        let diagram = SequenceDiagram {
            title: Some("Test".to_string()),
            participants: vec![
                Participant {
                    id: "A".to_string(),
                    label: "Alice".to_string(),
                },
                Participant {
                    id: "B".to_string(),
                    label: "Bob".to_string(),
                },
            ],
            messages: vec![Message {
                from: "A".to_string(),
                to: "B".to_string(),
                label: "Hello".to_string(),
                style: ArrowStyle::Solid,
            }],
        };
        let output = render_sequence_diagram(&diagram, &RenderOptions::default());
        assert!(output.contains("Test"));
        assert!(output.contains("Alice"));
        assert!(output.contains("Bob"));
        assert!(output.contains("Hello"));
    }
}
