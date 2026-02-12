//! Sequence diagram parser and renderer for Mermaid syntax
//!
//! Supports basic mermaid sequence diagram syntax

use std::collections::HashSet;

use winnow::ascii::{space0, space1};
use winnow::combinator::{alt, opt, preceded};
use winnow::token::{rest, take_while};
use winnow::PResult;
use winnow::Parser;

use crate::error::MermaidError;
use crate::text::display_width;
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
    /// Activate target participant after this message
    pub activate_to: bool,
    /// Deactivate target participant after this message
    pub deactivate_to: bool,
}

/// Note position relative to participants
#[derive(Debug, Clone)]
pub enum NotePosition {
    RightOf(String),
    LeftOf(String),
    Over(Vec<String>),
}

/// A note in the sequence diagram
#[derive(Debug, Clone)]
pub struct Note {
    pub position: NotePosition,
    pub text: String,
}

/// Fragment kind for interaction blocks
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FragmentKind {
    Loop,
    Alt,
    Opt,
    Par,
}

/// A section within a fragment (separated by else/and)
#[derive(Debug, Clone)]
pub struct FragmentSection {
    pub label: Option<String>,
    pub items: Vec<SequenceItem>,
}

/// An interaction fragment (loop, alt, opt, par)
#[derive(Debug, Clone)]
pub struct Fragment {
    pub kind: FragmentKind,
    pub label: String,
    pub sections: Vec<FragmentSection>,
}

/// Items in a sequence diagram (tree structure for nested fragments)
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum SequenceItem {
    Message(Message),
    Note(Note),
    Fragment(Fragment),
}

/// Sequence diagram data
#[derive(Debug, Clone)]
pub struct SequenceDiagram {
    pub title: Option<String>,
    pub participants: Vec<Participant>,
    pub messages: Vec<Message>,
    /// Whether to auto-number messages
    pub autonumber: bool,
    /// Notes attached after specific message indices (message_index, note)
    pub notes: Vec<(usize, Note)>,
    /// Active participant spans (participant_id, start_msg_idx, end_msg_idx)
    pub activations: Vec<(String, usize, usize)>,
    /// Tree-structured items (includes fragments)
    pub items: Vec<SequenceItem>,
}

/// Content of a single line
#[derive(Debug)]
enum SeqLine {
    Header,
    Title(String),
    AutoNumber,
    Participant {
        id: String,
        label: String,
    },
    Message(Message),
    Note(Note),
    Activate(String),
    Deactivate(String),
    /// Start of a fragment block: loop, alt, opt, par
    FragmentStart(FragmentKind, String),
    /// Section divider within a fragment: else, and
    FragmentDivider(Option<String>),
    /// End of a fragment block
    FragmentEnd,
    Empty,
}

/// Parse sequenceDiagram header
fn parse_header(input: &mut &str) -> PResult<()> {
    let _ = winnow::ascii::Caseless("sequencediagram").parse_next(input)?;
    Ok(())
}

/// Parse title declaration
fn parse_title(input: &mut &str) -> PResult<String> {
    let _ = winnow::ascii::Caseless("title").parse_next(input)?;
    let _ = space1.parse_next(input)?;
    let title = rest.parse_next(input)?;
    Ok(title.trim().to_string())
}

/// Parse autonumber directive
fn parse_autonumber(input: &mut &str) -> PResult<()> {
    let _ = winnow::ascii::Caseless("autonumber").parse_next(input)?;
    Ok(())
}

/// Parse participant/actor ID (alphanumeric and underscore only - no dash as it conflicts with arrows)
fn parse_participant_id(input: &mut &str) -> PResult<String> {
    take_while(1.., |c: char| c.is_alphanumeric() || c == '_')
        .map(|s: &str| s.to_string())
        .parse_next(input)
}

/// Parse target participant ID with optional +/- activation prefix
fn parse_target_participant_id(input: &mut &str) -> PResult<(String, bool, bool)> {
    let prefix = opt(alt(('+', '-'))).parse_next(input)?;
    let id = parse_participant_id(input)?;
    let activate = prefix == Some('+');
    let deactivate = prefix == Some('-');
    Ok((id, activate, deactivate))
}

/// Parse participant declaration: participant A as Alice or participant Alice
fn parse_participant_decl(input: &mut &str) -> PResult<(String, String)> {
    let _ = winnow::ascii::Caseless("participant").parse_next(input)?;
    let _ = space1.parse_next(input)?;
    let first_part = parse_participant_id.parse_next(input)?;

    // Check for "as" alias
    let _ = space0.parse_next(input)?;
    let alias = opt((
        winnow::ascii::Caseless("as"),
        space1,
        rest.map(|s: &str| s.trim().to_string()),
    ))
    .parse_next(input)?;

    if let Some((_, _, label)) = alias {
        Ok((first_part, label))
    } else {
        Ok((first_part.clone(), first_part))
    }
}

/// Parse actor declaration: actor A as Alice or actor Alice
fn parse_actor_decl(input: &mut &str) -> PResult<(String, String)> {
    let _ = winnow::ascii::Caseless("actor").parse_next(input)?;
    let _ = space1.parse_next(input)?;
    let first_part = parse_participant_id.parse_next(input)?;

    // Check for "as" alias
    let _ = space0.parse_next(input)?;
    let alias = opt((
        winnow::ascii::Caseless("as"),
        space1,
        rest.map(|s: &str| s.trim().to_string()),
    ))
    .parse_next(input)?;

    if let Some((_, _, label)) = alias {
        Ok((first_part, label))
    } else {
        Ok((first_part.clone(), first_part))
    }
}

/// Parse message arrow and extract style
fn parse_arrow(input: &mut &str) -> PResult<ArrowStyle> {
    alt((
        "-->>".map(|_| ArrowStyle::Dotted),
        "->>".map(|_| ArrowStyle::Solid),
        "-->".map(|_| ArrowStyle::DottedLine),
        "->".map(|_| ArrowStyle::SolidLine),
        "-)".map(|_| ArrowStyle::Async),
    ))
    .parse_next(input)
}

/// Parse message: From->>To: Label (with optional +/- on target for inline activation)
fn parse_message_line(input: &mut &str) -> PResult<Message> {
    let from = parse_participant_id.parse_next(input)?;
    let style = parse_arrow.parse_next(input)?;
    let (to, activate_to, deactivate_to) = parse_target_participant_id(input)?;

    // Optional label after colon
    let _ = space0.parse_next(input)?;
    let label = opt(preceded(':', preceded(space0, rest)))
        .map(|o: Option<&str>| o.map(|s| s.trim().to_string()).unwrap_or_default())
        .parse_next(input)?;

    Ok(Message {
        from,
        to,
        label,
        style,
        activate_to,
        deactivate_to,
    })
}

/// Parse note line: Note right of A: text, Note left of A: text, Note over A,B: text
fn parse_note_line(line: &str) -> Option<Note> {
    let lower = line.to_lowercase();
    if !lower.starts_with("note ") {
        return None;
    }
    let rest = line[5..].trim();

    // Find the colon separator for text
    let colon_idx = rest.find(':')?;
    let position_part = rest[..colon_idx].trim();
    let text = rest[colon_idx + 1..].trim().to_string();

    let lower_pos = position_part.to_lowercase();

    let position = if lower_pos.starts_with("right of ") {
        let id = position_part[9..].trim().to_string();
        NotePosition::RightOf(id)
    } else if lower_pos.starts_with("left of ") {
        let id = position_part[8..].trim().to_string();
        NotePosition::LeftOf(id)
    } else if lower_pos.starts_with("over ") {
        let ids_str = position_part[5..].trim();
        let ids: Vec<String> = ids_str.split(',').map(|s| s.trim().to_string()).collect();
        NotePosition::Over(ids)
    } else {
        return None;
    };

    Some(Note { position, text })
}

/// Parse activate/deactivate line
fn parse_activate_line(line: &str) -> Option<(bool, String)> {
    let lower = line.to_lowercase();
    if lower.starts_with("activate ") {
        Some((true, line[9..].trim().to_string()))
    } else if lower.starts_with("deactivate ") {
        Some((false, line[11..].trim().to_string()))
    } else {
        None
    }
}

/// Parse a single line and classify it
fn parse_line(line: &str) -> SeqLine {
    let trimmed = line.trim();

    // Empty or comment
    if trimmed.is_empty() || trimmed.starts_with("%%") {
        return SeqLine::Empty;
    }

    // Header
    if parse_header.parse(trimmed).is_ok() {
        return SeqLine::Header;
    }

    // AutoNumber
    if parse_autonumber.parse(trimmed).is_ok() {
        return SeqLine::AutoNumber;
    }

    // Title
    if let Ok(title) = parse_title.parse(trimmed) {
        return SeqLine::Title(title);
    }

    // Participant
    if let Ok((id, label)) = parse_participant_decl.parse(trimmed) {
        return SeqLine::Participant { id, label };
    }

    // Actor
    if let Ok((id, label)) = parse_actor_decl.parse(trimmed) {
        return SeqLine::Participant { id, label };
    }

    // Fragment end
    let lower = trimmed.to_lowercase();
    if lower == "end" {
        return SeqLine::FragmentEnd;
    }

    // Fragment start: loop, alt, opt, par
    if lower.starts_with("loop ") || lower == "loop" {
        let label = if trimmed.len() > 5 {
            trimmed[5..].trim().to_string()
        } else {
            String::new()
        };
        return SeqLine::FragmentStart(FragmentKind::Loop, label);
    }
    if lower.starts_with("alt ") || lower == "alt" {
        let label = if trimmed.len() > 4 {
            trimmed[4..].trim().to_string()
        } else {
            String::new()
        };
        return SeqLine::FragmentStart(FragmentKind::Alt, label);
    }
    if lower.starts_with("opt ") || lower == "opt" {
        let label = if trimmed.len() > 4 {
            trimmed[4..].trim().to_string()
        } else {
            String::new()
        };
        return SeqLine::FragmentStart(FragmentKind::Opt, label);
    }
    if lower.starts_with("par ") || lower == "par" {
        let label = if trimmed.len() > 4 {
            trimmed[4..].trim().to_string()
        } else {
            String::new()
        };
        return SeqLine::FragmentStart(FragmentKind::Par, label);
    }

    // Fragment dividers: else, and
    if lower.starts_with("else ") || lower == "else" {
        let label = if trimmed.len() > 5 {
            Some(trimmed[5..].trim().to_string())
        } else {
            None
        };
        return SeqLine::FragmentDivider(label);
    }
    if lower.starts_with("and ") || lower == "and" {
        let label = if trimmed.len() > 4 {
            Some(trimmed[4..].trim().to_string())
        } else {
            None
        };
        return SeqLine::FragmentDivider(label);
    }

    // Note
    if let Some(note) = parse_note_line(trimmed) {
        return SeqLine::Note(note);
    }

    // Activate/Deactivate
    if let Some((is_activate, id)) = parse_activate_line(trimmed) {
        return if is_activate {
            SeqLine::Activate(id)
        } else {
            SeqLine::Deactivate(id)
        };
    }

    // Message (with inline +/- activation support handled in parser)
    if let Ok(msg) = parse_message_line.parse(trimmed) {
        return SeqLine::Message(msg);
    }

    SeqLine::Empty
}

/// Parse sequence diagram syntax
pub fn parse_sequence_diagram(input: &str) -> Result<SequenceDiagram, MermaidError> {
    let lines: Vec<&str> = input.lines().collect();

    if lines.is_empty() || lines.iter().all(|l| l.trim().is_empty()) {
        return Err(MermaidError::EmptyInput);
    }

    let mut diagram = SequenceDiagram {
        title: None,
        participants: Vec::new(),
        messages: Vec::new(),
        autonumber: false,
        notes: Vec::new(),
        activations: Vec::new(),
        items: Vec::new(),
    };

    let mut seen_participants: HashSet<String> = HashSet::new();
    let mut found_header = false;
    // Track pending activations: participant_id -> start message index
    let mut active_stack: std::collections::HashMap<String, Vec<usize>> =
        std::collections::HashMap::new();

    // Stack for building nested fragments
    // Each entry: (FragmentKind, label, sections_so_far, current_section_label, current_section_items)
    struct FragmentBuilder {
        kind: FragmentKind,
        label: String,
        sections: Vec<FragmentSection>,
        current_label: Option<String>,
        current_items: Vec<SequenceItem>,
    }
    let mut fragment_stack: Vec<FragmentBuilder> = Vec::new();

    // Helper closure to get mutable reference to current items list
    // (either top-level items or current fragment section)
    fn push_item(
        diagram_items: &mut Vec<SequenceItem>,
        stack: &mut [FragmentBuilder],
        item: SequenceItem,
    ) {
        if let Some(builder) = stack.last_mut() {
            builder.current_items.push(item);
        } else {
            diagram_items.push(item);
        }
    }

    for line in lines.iter() {
        match parse_line(line) {
            SeqLine::Header => {
                found_header = true;
            }
            SeqLine::Title(t) => {
                diagram.title = Some(t);
            }
            SeqLine::AutoNumber => {
                diagram.autonumber = true;
            }
            SeqLine::Participant { id, label } => {
                if !seen_participants.contains(&id) {
                    seen_participants.insert(id.clone());
                    diagram.participants.push(Participant { id, label });
                }
            }
            SeqLine::Note(note) => {
                // Attach to current message count (after the last message)
                let idx = diagram.messages.len().saturating_sub(1);
                diagram.notes.push((idx, note.clone()));
                push_item(
                    &mut diagram.items,
                    &mut fragment_stack,
                    SequenceItem::Note(note),
                );
            }
            SeqLine::Activate(id) => {
                active_stack
                    .entry(id)
                    .or_default()
                    .push(diagram.messages.len());
            }
            SeqLine::Deactivate(id) => {
                if let Some(starts) = active_stack.get_mut(&id) {
                    if let Some(start) = starts.pop() {
                        diagram
                            .activations
                            .push((id, start, diagram.messages.len()));
                    }
                }
            }
            SeqLine::FragmentStart(kind, label) => {
                fragment_stack.push(FragmentBuilder {
                    kind,
                    label,
                    sections: Vec::new(),
                    current_label: None,
                    current_items: Vec::new(),
                });
            }
            SeqLine::FragmentDivider(label) => {
                if let Some(builder) = fragment_stack.last_mut() {
                    // Close current section and start new one
                    let prev_items = std::mem::take(&mut builder.current_items);
                    let prev_label = builder.current_label.take();
                    builder.sections.push(FragmentSection {
                        label: prev_label,
                        items: prev_items,
                    });
                    builder.current_label = label;
                }
            }
            SeqLine::FragmentEnd => {
                if let Some(mut builder) = fragment_stack.pop() {
                    // Close the last section
                    builder.sections.push(FragmentSection {
                        label: builder.current_label,
                        items: builder.current_items,
                    });
                    let fragment = Fragment {
                        kind: builder.kind,
                        label: builder.label,
                        sections: builder.sections,
                    };
                    push_item(
                        &mut diagram.items,
                        &mut fragment_stack,
                        SequenceItem::Fragment(fragment),
                    );
                }
            }
            SeqLine::Message(msg) => {
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
                // Handle inline activation/deactivation
                let activate_to = msg.activate_to;
                let deactivate_to = msg.deactivate_to;
                let to_id = msg.to.clone();
                // Capture index BEFORE push for correct activation range
                let msg_idx = diagram.messages.len();
                push_item(
                    &mut diagram.items,
                    &mut fragment_stack,
                    SequenceItem::Message(msg.clone()),
                );
                diagram.messages.push(msg);
                if activate_to {
                    active_stack.entry(to_id.clone()).or_default().push(msg_idx);
                }
                if deactivate_to {
                    if let Some(starts) = active_stack.get_mut(&to_id) {
                        if let Some(start) = starts.pop() {
                            diagram.activations.push((to_id, start, msg_idx + 1));
                        }
                    }
                }
            }
            SeqLine::Empty => {}
        }
    }

    // Close any unclosed activations
    let total_msgs = diagram.messages.len();
    for (id, starts) in &active_stack {
        for &start in starts {
            diagram.activations.push((id.clone(), start, total_msgs));
        }
    }

    // Close any unclosed fragments
    while let Some(mut builder) = fragment_stack.pop() {
        builder.sections.push(FragmentSection {
            label: builder.current_label,
            items: builder.current_items,
        });
        let fragment = Fragment {
            kind: builder.kind,
            label: builder.label,
            sections: builder.sections,
        };
        push_item(
            &mut diagram.items,
            &mut fragment_stack,
            SequenceItem::Fragment(fragment),
        );
    }

    if !found_header {
        return Err(MermaidError::ParseError {
            line: 1,
            message: "Expected 'sequenceDiagram'".to_string(),
            suggestion: Some("Start with 'sequenceDiagram'".to_string()),
        });
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
    let active_v = if options.ascii { '#' } else { '┃' };

    // Calculate participant column widths
    let min_col_width = 12;
    let col_widths: Vec<usize> = diagram
        .participants
        .iter()
        .map(|p| (display_width(&p.label) + 4).max(min_col_width))
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
        let title_w = display_width(title);
        let padding = (total_width.saturating_sub(title_w)) / 2;
        output.push_str(&" ".repeat(padding));
        output.push_str(title);
        output.push('\n');
        output.push_str(&" ".repeat(padding));
        output.push_str(&"─".repeat(title_w));
        output.push_str("\n\n");
    }

    // Draw participant boxes at top
    // Box top line
    let mut line = vec![' '; total_width];
    for (i, p) in diagram.participants.iter().enumerate() {
        let center = positions[i];
        let box_width = display_width(&p.label) + 2;
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
        let box_width = display_width(&p.label) + 2;
        let start = center.saturating_sub(box_width / 2);
        let end = start + box_width;

        if start < total_width {
            line[start] = box_v;
        }
        // Center label (advance by display width for CJK support)
        let label_start = start + 1;
        let mut dx = 0;
        for c in p.label.chars() {
            if label_start + dx < total_width {
                line[label_start + dx] = c;
            }
            dx += unicode_width::UnicodeWidthChar::width(c).unwrap_or(1);
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
        let box_width = display_width(&p.label) + 2;
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

    // Collect fragment spans for rendering
    // Each span: (start_msg_idx, end_msg_idx, kind_label, section_labels)
    struct FragmentSpan {
        kind: FragmentKind,
        label: String,
        start_msg: usize,
        end_msg: usize,
        dividers: Vec<(usize, Option<String>)>, // (msg_idx, label) for else/and lines
    }

    fn collect_fragment_spans(
        items: &[SequenceItem],
        msg_counter: &mut usize,
        spans: &mut Vec<FragmentSpan>,
    ) {
        for item in items {
            match item {
                SequenceItem::Message(_) => {
                    *msg_counter += 1;
                }
                SequenceItem::Note(_) => {}
                SequenceItem::Fragment(frag) => {
                    let start = *msg_counter;
                    let mut dividers = Vec::new();
                    for (si, section) in frag.sections.iter().enumerate() {
                        if si > 0 {
                            dividers.push((*msg_counter, section.label.clone()));
                        }
                        collect_fragment_spans(&section.items, msg_counter, spans);
                    }
                    spans.push(FragmentSpan {
                        kind: frag.kind.clone(),
                        label: frag.label.clone(),
                        start_msg: start,
                        end_msg: *msg_counter,
                        dividers,
                    });
                }
            }
        }
    }

    let mut fragment_spans = Vec::new();
    let mut msg_counter = 0;
    collect_fragment_spans(&diagram.items, &mut msg_counter, &mut fragment_spans);

    // Helper: check if participant is active at a given message index
    let is_active = |participant_id: &str, at_msg: usize| -> bool {
        diagram
            .activations
            .iter()
            .any(|(id, start, end)| id == participant_id && at_msg >= *start && at_msg < *end)
    };

    // Helper: get lifeline char for a participant at a given message index
    let lifeline_char = |p_idx: usize, at_msg: usize| -> char {
        let pid = &diagram.participants[p_idx].id;
        if is_active(pid, at_msg) {
            active_v
        } else if options.ascii {
            '|'
        } else {
            '│'
        }
    };

    let (frag_h, frag_v, frag_tl, frag_tr, frag_bl, frag_br, frag_dashed) = if options.ascii {
        ('-', '|', '+', '+', '+', '+', '-')
    } else {
        ('─', '│', '┌', '┐', '└', '┘', '╌')
    };

    // Helper to draw a fragment top border with label
    let draw_fragment_top = |output: &mut String,
                             total_width: usize,
                             positions: &[usize],
                             kind: &FragmentKind,
                             label: &str,
                             lifeline_fn: &dyn Fn(usize, usize) -> char,
                             msg_idx: usize| {
        let kind_str = match kind {
            FragmentKind::Loop => "loop",
            FragmentKind::Alt => "alt",
            FragmentKind::Opt => "opt",
            FragmentKind::Par => "par",
        };
        let tag = if label.is_empty() {
            format!("[{}]", kind_str)
        } else {
            format!("[{} {}]", kind_str, label)
        };
        let frag_width = total_width.saturating_sub(2);

        // Top border line
        let mut line = vec![' '; total_width];
        for (pi, &pos) in positions.iter().enumerate() {
            if pos < total_width {
                line[pos] = lifeline_fn(pi, msg_idx);
            }
        }
        // Draw top border over lifelines
        if frag_width > 0 {
            line[1] = frag_tl;
            for i in 2..total_width.saturating_sub(1) {
                line[i] = frag_h;
            }
            if total_width > 2 {
                line[total_width - 2] = frag_tr;
            }
        }
        // Overlay the tag
        for (i, c) in tag.chars().enumerate() {
            if 2 + i < total_width - 2 {
                line[2 + i] = c;
            }
        }
        output.push_str(line.iter().collect::<String>().trim_end());
        output.push('\n');
    };

    // Helper to draw a fragment section divider (dashed line for else/and)
    let draw_fragment_divider = |output: &mut String,
                                 total_width: usize,
                                 positions: &[usize],
                                 label: &Option<String>,
                                 lifeline_fn: &dyn Fn(usize, usize) -> char,
                                 msg_idx: usize| {
        let mut line = vec![' '; total_width];
        for (pi, &pos) in positions.iter().enumerate() {
            if pos < total_width {
                line[pos] = lifeline_fn(pi, msg_idx);
            }
        }
        // Dashed line
        if total_width > 3 {
            line[1] = frag_v;
            for i in 2..total_width.saturating_sub(2) {
                line[i] = frag_dashed;
            }
            line[total_width - 2] = frag_v;
        }
        // Overlay label if any
        if let Some(lbl) = label {
            let tag = format!("[{}]", lbl);
            for (i, c) in tag.chars().enumerate() {
                if 2 + i < total_width - 2 {
                    line[2 + i] = c;
                }
            }
        }
        output.push_str(line.iter().collect::<String>().trim_end());
        output.push('\n');
    };

    // Helper to draw a fragment bottom border
    let draw_fragment_bottom = |output: &mut String,
                                total_width: usize,
                                positions: &[usize],
                                lifeline_fn: &dyn Fn(usize, usize) -> char,
                                msg_idx: usize| {
        let mut line = vec![' '; total_width];
        for (pi, &pos) in positions.iter().enumerate() {
            if pos < total_width {
                line[pos] = lifeline_fn(pi, msg_idx);
            }
        }
        if total_width > 3 {
            line[1] = frag_bl;
            for i in 2..total_width.saturating_sub(2) {
                line[i] = frag_h;
            }
            line[total_width - 2] = frag_br;
        }
        output.push_str(line.iter().collect::<String>().trim_end());
        output.push('\n');
    };

    // Draw vertical lines (lifelines) and messages
    for (msg_idx, msg) in diagram.messages.iter().enumerate() {
        // Draw fragment starts at this message index
        for span in &fragment_spans {
            if span.start_msg == msg_idx {
                draw_fragment_top(
                    &mut output,
                    total_width,
                    &positions,
                    &span.kind,
                    &span.label,
                    &lifeline_char,
                    msg_idx,
                );
            }
        }
        // Draw fragment dividers at this message index
        for span in &fragment_spans {
            for (div_idx, div_label) in &span.dividers {
                if *div_idx == msg_idx {
                    draw_fragment_divider(
                        &mut output,
                        total_width,
                        &positions,
                        div_label,
                        &lifeline_char,
                        msg_idx,
                    );
                }
            }
        }

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

            // Self-message loop (same participant)
            if from_i == to_i {
                let loop_width = 4;
                let (h_line, corner_tl, corner_tr, corner_bl, corner_br) = if options.ascii {
                    ('-', '+', '+', '+', '+')
                } else {
                    ('─', '╭', '╮', '╰', '╯')
                };

                // Row 1: lifelines + top of loop
                let mut line = vec![' '; total_width + loop_width + 2];
                for (pi, &pos) in positions.iter().enumerate() {
                    if pos < line.len() {
                        line[pos] = lifeline_char(pi, msg_idx);
                    }
                }
                // Draw top of loop: ╭──╮
                if from_x + 1 < line.len() {
                    line[from_x + 1] = corner_tl;
                }
                for i in 2..=loop_width {
                    if from_x + i < line.len() {
                        line[from_x + i] = h_line;
                    }
                }
                if from_x + loop_width + 1 < line.len() {
                    line[from_x + loop_width + 1] = corner_tr;
                }
                output.push_str(line.iter().collect::<String>().trim_end());
                output.push('\n');

                // Row 2: lifelines + vertical sides
                let mut line = vec![' '; total_width + loop_width + 2];
                for (pi, &pos) in positions.iter().enumerate() {
                    if pos < line.len() {
                        line[pos] = lifeline_char(pi, msg_idx);
                    }
                }
                if from_x + 1 < line.len() {
                    line[from_x + 1] = if options.ascii { '|' } else { '│' };
                }
                if from_x + loop_width + 1 < line.len() {
                    line[from_x + loop_width + 1] = if options.ascii { '|' } else { '│' };
                }
                output.push_str(line.iter().collect::<String>().trim_end());
                // Add label
                if diagram.autonumber || !msg.label.is_empty() {
                    output.push_str("  ");
                    if diagram.autonumber {
                        output.push_str(&format!("{}. ", msg_idx + 1));
                    }
                    output.push_str(&msg.label);
                }
                output.push('\n');

                // Row 3: lifelines + bottom of loop with arrow
                let mut line = vec![' '; total_width + loop_width + 2];
                for (pi, &pos) in positions.iter().enumerate() {
                    if pos < line.len() {
                        line[pos] = lifeline_char(pi, msg_idx);
                    }
                }
                if from_x + 1 < line.len() {
                    line[from_x + 1] = corner_bl;
                }
                // Arrow pointing back
                if from_x + 2 < line.len() {
                    line[from_x + 2] = arrow_l;
                }
                for i in 3..=loop_width {
                    if from_x + i < line.len() {
                        line[from_x + i] = h_line;
                    }
                }
                if from_x + loop_width + 1 < line.len() {
                    line[from_x + loop_width + 1] = corner_br;
                }
                output.push_str(line.iter().collect::<String>().trim_end());
                output.push('\n');

                continue;
            }

            // Draw lifeline row with vertical lines at participant positions
            let mut line = vec![' '; total_width];
            for (pi, &pos) in positions.iter().enumerate() {
                if pos < total_width {
                    line[pos] = lifeline_char(pi, msg_idx);
                }
            }
            output.push_str(&line.iter().collect::<String>());
            output.push('\n');

            // Draw message arrow
            let mut line = vec![' '; total_width];
            for (pi, &pos) in positions.iter().enumerate() {
                if pos < total_width {
                    line[pos] = lifeline_char(pi, msg_idx);
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

            // Add label (with optional autonumber prefix)
            if diagram.autonumber || !msg.label.is_empty() {
                output.push_str("  ");
                if diagram.autonumber {
                    output.push_str(&format!("{}. ", msg_idx + 1));
                }
                output.push_str(&msg.label);
            }
            output.push('\n');
        }

        // Draw notes attached to this message
        for (note_idx, note) in &diagram.notes {
            if *note_idx != msg_idx {
                continue;
            }
            let note_text = &note.text;
            let note_width = display_width(note_text) + 4; // "│ text │"

            // Determine note x position based on NotePosition
            let note_x = match &note.position {
                NotePosition::RightOf(id) => {
                    let p_idx = diagram
                        .participants
                        .iter()
                        .position(|p| p.id == *id || p.label == *id);
                    if let Some(pi) = p_idx {
                        positions[pi] + 2
                    } else {
                        0
                    }
                }
                NotePosition::LeftOf(id) => {
                    let p_idx = diagram
                        .participants
                        .iter()
                        .position(|p| p.id == *id || p.label == *id);
                    if let Some(pi) = p_idx {
                        positions[pi].saturating_sub(note_width + 1)
                    } else {
                        0
                    }
                }
                NotePosition::Over(ids) => {
                    let indices: Vec<usize> = ids
                        .iter()
                        .filter_map(|id| {
                            diagram
                                .participants
                                .iter()
                                .position(|p| p.id == *id || p.label == *id)
                        })
                        .collect();
                    if indices.is_empty() {
                        0
                    } else {
                        let min_x = indices.iter().map(|&i| positions[i]).min().unwrap();
                        let max_x = indices.iter().map(|&i| positions[i]).max().unwrap();
                        let center = (min_x + max_x) / 2;
                        center.saturating_sub(note_width / 2)
                    }
                }
            };

            let render_width = total_width.max(note_x + note_width + 1);

            // Note top border
            let mut nline = vec![' '; render_width];
            for &pos in &positions {
                if pos < nline.len() {
                    nline[pos] = if options.ascii { '|' } else { '│' };
                }
            }
            if note_x < nline.len() {
                nline[note_x] = box_tl;
            }
            for i in 1..note_width - 1 {
                if note_x + i < nline.len() {
                    nline[note_x + i] = box_h;
                }
            }
            if note_x + note_width - 1 < nline.len() {
                nline[note_x + note_width - 1] = box_tr;
            }
            output.push_str(nline.iter().collect::<String>().trim_end());
            output.push('\n');

            // Note content
            let mut nline = vec![' '; render_width];
            for &pos in &positions {
                if pos < nline.len() {
                    nline[pos] = if options.ascii { '|' } else { '│' };
                }
            }
            if note_x < nline.len() {
                nline[note_x] = box_v;
            }
            let text_start = note_x + 2;
            for (i, c) in note_text.chars().enumerate() {
                if text_start + i < nline.len() {
                    nline[text_start + i] = c;
                }
            }
            if note_x + note_width - 1 < nline.len() {
                nline[note_x + note_width - 1] = box_v;
            }
            output.push_str(nline.iter().collect::<String>().trim_end());
            output.push('\n');

            // Note bottom border
            let mut nline = vec![' '; render_width];
            for &pos in &positions {
                if pos < nline.len() {
                    nline[pos] = if options.ascii { '|' } else { '│' };
                }
            }
            if note_x < nline.len() {
                nline[note_x] = box_bl;
            }
            for i in 1..note_width - 1 {
                if note_x + i < nline.len() {
                    nline[note_x + i] = box_h;
                }
            }
            if note_x + note_width - 1 < nline.len() {
                nline[note_x + note_width - 1] = box_br;
            }
            output.push_str(nline.iter().collect::<String>().trim_end());
            output.push('\n');
        }

        // Draw fragment ends after this message
        let next_msg = msg_idx + 1;
        for span in &fragment_spans {
            if span.end_msg == next_msg {
                draw_fragment_bottom(
                    &mut output,
                    total_width,
                    &positions,
                    &lifeline_char,
                    msg_idx,
                );
            }
        }
    }

    // Final lifeline row
    let total_msgs = diagram.messages.len();
    let mut line = vec![' '; total_width];
    for (pi, &pos) in positions.iter().enumerate() {
        if pos < total_width {
            line[pos] = lifeline_char(pi, total_msgs);
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
                activate_to: false,
                deactivate_to: false,
            }],
            autonumber: false,
            notes: Vec::new(),
            activations: Vec::new(),
            items: Vec::new(),
        };
        let output = render_sequence_diagram(&diagram, &RenderOptions::default());
        assert!(output.contains("Test"));
        assert!(output.contains("Alice"));
        assert!(output.contains("Bob"));
        assert!(output.contains("Hello"));
    }

    #[test]
    fn test_parse_autonumber() {
        let input = r#"sequenceDiagram
    autonumber
    Alice->>Bob: Hello
    Bob->>Alice: Hi
"#;
        let diagram = parse_sequence_diagram(input).unwrap();
        assert!(diagram.autonumber);
        assert_eq!(diagram.messages.len(), 2);
    }

    #[test]
    fn test_render_autonumber() {
        let input = r#"sequenceDiagram
    autonumber
    Alice->>Bob: Hello
    Bob->>Alice: Hi
"#;
        let diagram = parse_sequence_diagram(input).unwrap();
        let output = render_sequence_diagram(&diagram, &RenderOptions::default());
        assert!(output.contains("1. Hello"));
        assert!(output.contains("2. Hi"));
    }

    #[test]
    fn test_self_message_loop() {
        let input = r#"sequenceDiagram
    Alice->>Alice: Think
"#;
        let diagram = parse_sequence_diagram(input).unwrap();
        assert_eq!(diagram.messages.len(), 1);
        assert_eq!(diagram.messages[0].from, "Alice");
        assert_eq!(diagram.messages[0].to, "Alice");

        let output = render_sequence_diagram(&diagram, &RenderOptions::default());
        assert!(output.contains("Think"));
        // Should contain loop characters
        assert!(output.contains("╭") || output.contains("+"));
    }

    #[test]
    fn test_parse_arrow() {
        assert_eq!(parse_arrow.parse("-->>").unwrap(), ArrowStyle::Dotted);
        assert_eq!(parse_arrow.parse("->>").unwrap(), ArrowStyle::Solid);
        assert_eq!(parse_arrow.parse("-->").unwrap(), ArrowStyle::DottedLine);
        assert_eq!(parse_arrow.parse("->").unwrap(), ArrowStyle::SolidLine);
        assert_eq!(parse_arrow.parse("-)").unwrap(), ArrowStyle::Async);
    }

    #[test]
    fn test_parse_note_right_of() {
        let input = r#"sequenceDiagram
    Alice->>Bob: Hello
    Note right of Bob: Think about it
"#;
        let diagram = parse_sequence_diagram(input).unwrap();
        assert_eq!(diagram.notes.len(), 1);
        assert!(matches!(
            &diagram.notes[0].1.position,
            NotePosition::RightOf(id) if id == "Bob"
        ));
        assert_eq!(diagram.notes[0].1.text, "Think about it");
    }

    #[test]
    fn test_parse_note_over() {
        let input = r#"sequenceDiagram
    Alice->>Bob: Hello
    Note over Alice,Bob: Shared note
"#;
        let diagram = parse_sequence_diagram(input).unwrap();
        assert_eq!(diagram.notes.len(), 1);
        assert!(matches!(
            &diagram.notes[0].1.position,
            NotePosition::Over(ids) if ids.len() == 2
        ));
    }

    #[test]
    fn test_render_note() {
        let input = r#"sequenceDiagram
    Alice->>Bob: Hello
    Note right of Bob: Important
"#;
        let diagram = parse_sequence_diagram(input).unwrap();
        let output = render_sequence_diagram(&diagram, &RenderOptions::default());
        assert!(output.contains("Important"));
        // Note box borders
        assert!(output.contains("┌") || output.contains("+"));
    }

    #[test]
    fn test_parse_activate_deactivate() {
        let input = r#"sequenceDiagram
    Alice->>Bob: Hello
    activate Bob
    Bob->>Alice: Hi
    deactivate Bob
"#;
        let diagram = parse_sequence_diagram(input).unwrap();
        assert_eq!(diagram.activations.len(), 1);
        assert_eq!(diagram.activations[0].0, "Bob");
    }

    #[test]
    fn test_parse_inline_activation() {
        let input = r#"sequenceDiagram
    Alice->>+Bob: Hello
    Bob->>-Alice: Bye
"#;
        let diagram = parse_sequence_diagram(input).unwrap();
        assert_eq!(diagram.messages[0].activate_to, true);
        assert_eq!(diagram.messages[0].to, "Bob");
        assert_eq!(diagram.messages[1].deactivate_to, true);
        assert_eq!(diagram.activations.len(), 1);
    }

    #[test]
    fn test_render_activation_box() {
        let input = r#"sequenceDiagram
    Alice->>+Bob: Hello
    Bob->>-Alice: Bye
"#;
        let diagram = parse_sequence_diagram(input).unwrap();
        let output = render_sequence_diagram(&diagram, &RenderOptions::default());
        // Active lifelines use ┃ instead of │
        assert!(output.contains('┃'));
    }

    #[test]
    fn test_parse_loop_fragment() {
        let input = r#"sequenceDiagram
    Alice->>Bob: Hello
    loop Every minute
        Bob->>Alice: Ping
    end
"#;
        let diagram = parse_sequence_diagram(input).unwrap();
        assert_eq!(diagram.messages.len(), 2);
        assert_eq!(diagram.items.len(), 2); // Message + Fragment
        if let SequenceItem::Fragment(frag) = &diagram.items[1] {
            assert_eq!(frag.kind, FragmentKind::Loop);
            assert_eq!(frag.label, "Every minute");
            assert_eq!(frag.sections.len(), 1);
        } else {
            panic!("Expected Fragment");
        }
    }

    #[test]
    fn test_parse_alt_fragment() {
        let input = r#"sequenceDiagram
    Alice->>Bob: Request
    alt Success
        Bob->>Alice: OK
    else Failure
        Bob->>Alice: Error
    end
"#;
        let diagram = parse_sequence_diagram(input).unwrap();
        assert_eq!(diagram.messages.len(), 3);
        if let SequenceItem::Fragment(frag) = &diagram.items[1] {
            assert_eq!(frag.kind, FragmentKind::Alt);
            assert_eq!(frag.sections.len(), 2);
            assert_eq!(frag.sections[1].label, Some("Failure".to_string()));
        } else {
            panic!("Expected Fragment");
        }
    }

    #[test]
    fn test_render_loop_fragment() {
        let input = r#"sequenceDiagram
    Alice->>Bob: Hello
    loop Every minute
        Bob->>Alice: Ping
    end
"#;
        let diagram = parse_sequence_diagram(input).unwrap();
        let output = render_sequence_diagram(&diagram, &RenderOptions::default());
        assert!(output.contains("[loop Every minute]"));
    }

    #[test]
    fn test_render_alt_fragment() {
        let input = r#"sequenceDiagram
    alt Success
        Alice->>Bob: OK
    else Failure
        Alice->>Bob: Error
    end
"#;
        let diagram = parse_sequence_diagram(input).unwrap();
        let output = render_sequence_diagram(&diagram, &RenderOptions::default());
        assert!(output.contains("[alt Success]"));
        assert!(output.contains("[Failure]"));
    }

    #[test]
    fn test_unclosed_activation_extends_to_end() {
        let input = r#"sequenceDiagram
    activate Alice
    Alice->>Bob: Hello
    Bob->>Alice: Hi
"#;
        let diagram = parse_sequence_diagram(input).unwrap();
        assert_eq!(diagram.activations.len(), 1);
        assert_eq!(diagram.activations[0].0, "Alice");
        // Should extend to end (total messages = 2)
        assert_eq!(diagram.activations[0].2, 2);
    }
}
