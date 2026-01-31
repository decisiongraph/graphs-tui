# graphs-tui

Terminal renderer for **Mermaid** and **D2** diagrams in Rust.

Render flowcharts, state diagrams, pie charts, and D2 diagrams as clean Unicode or ASCII text in your terminal. Zero dependencies.

## Features

- **Mermaid Support**: Flowcharts, state diagrams, pie charts
- **D2 Support**: Shapes, connections, containers, edge labels
- **Unicode & ASCII**: Beautiful Unicode boxes by default, ASCII fallback
- **Auto-Detection**: Automatically detects Mermaid vs D2 format
- **Zero Dependencies**: Pure Rust, no external crates

## Installation

```toml
[dependencies]
graphs-tui = "0.1"
```

## Examples

### Mermaid Flowchart

```rust
use graphs_tui::{render_mermaid_to_tui, RenderOptions};

let input = r#"flowchart LR
    A[Start] --> B[Process]
    B --> C{Decision}
    C -->|Yes| D[Done]
    C -->|No| B"#;

let output = render_mermaid_to_tui(input, RenderOptions::default()).unwrap();
println!("{}", output);
```

**Output:**
```
┌─────┐        ┌───────┐        /  \
│Start│───────▶│Process│───────▶<Dec>
└─────┘        └───────┘        \  /
                                 │
                    ┌────Yes─────┘
                    ▼
                 ┌────┐
                 │Done│
                 └────┘
```

### Mermaid State Diagram

```rust
use graphs_tui::{render_state_diagram, RenderOptions};

let input = r#"stateDiagram-v2
    [*] --> Idle
    Idle --> Running: start
    Running --> Idle: stop
    Running --> [*]: complete"#;

let output = render_state_diagram(input, RenderOptions::default()).unwrap();
println!("{}", output);
```

**Output:**
```
  (●)
   │
   ▼
╭────╮
│Idle│◀──stop──┐
╰────╯         │
   │           │
   │start      │
   ▼           │
╭───────╮      │
│Running│──────┘
╰───────╯
   │
   │complete
   ▼
  (◉)
```

### Mermaid Pie Chart

```rust
use graphs_tui::{render_pie_chart, RenderOptions};

let input = r#"pie
    title Project Status
    "Completed" : 45
    "In Progress" : 30
    "Pending" : 15
    "Blocked" : 10"#;

let output = render_pie_chart(input, RenderOptions::default()).unwrap();
println!("{}", output);
```

**Output:**
```
  Project Status
  ──────────────

  Completed    │██████████████                │ 45 (45.0%)
  In Progress  │█████████                     │ 30 (30.0%)
  Pending      │▒▒▒▒▒                         │ 15 (15.0%)
  Blocked      │▒▒▒                           │ 10 (10.0%)

               Total: 100
```

### D2 Diagram

```rust
use graphs_tui::{render_d2_to_tui, RenderOptions};

let input = r#"
user: User
server: Web Server
db: Database

user -> server: HTTP request
server -> db: SQL query
db -> server: Result
server -> user: Response
"#;

let output = render_d2_to_tui(input, RenderOptions::default()).unwrap();
println!("{}", output);
```

**Output:**
```
┌────┐
│User│
└────┘
   │
   │HTTP request
   ▼
┌──────────┐
│Web Server│
└──────────┘
   │
   │SQL query
   ▼
┌────────┐
│Database│
└────────┘
```

### D2 with Containers

```rust
use graphs_tui::{render_d2_to_tui, RenderOptions};

let input = r#"
backend {
    api: API Server
    db: Database
    cache: Redis
}

frontend {
    web: React App
    mobile: iOS App
}

web -> api
mobile -> api
api -> db
api -> cache
"#;

let output = render_d2_to_tui(input, RenderOptions::default()).unwrap();
println!("{}", output);
```

### Auto-Detection

```rust
use graphs_tui::{render_diagram, detect_format, DiagramFormat, RenderOptions};

let mermaid_input = "flowchart LR\n    A --> B --> C";
let d2_input = "A -> B -> C";

// Auto-detect and render
let output1 = render_diagram(mermaid_input, RenderOptions::default()).unwrap();
let output2 = render_diagram(d2_input, RenderOptions::default()).unwrap();

// Check format
assert_eq!(detect_format(mermaid_input), DiagramFormat::Mermaid);
assert_eq!(detect_format(d2_input), DiagramFormat::D2);
```

### ASCII Mode

For environments without Unicode support:

```rust
use graphs_tui::{render_mermaid_to_tui, RenderOptions};

let input = "flowchart LR\n    A[Start] --> B[End]";
let options = RenderOptions { ascii: true, max_width: None };
let output = render_mermaid_to_tui(input, options).unwrap();
println!("{}", output);
```

**Output:**
```
+-----+        +---+
|Start|------->|End|
+-----+        +---+
```

## Supported Syntax

### Mermaid Flowcharts

| Feature | Syntax | Example |
|---------|--------|---------|
| Directions | `LR`, `RL`, `TB`, `BT` | `flowchart LR` |
| Rectangle | `[label]` | `A[My Node]` |
| Rounded | `(label)` | `A(Rounded)` |
| Circle | `((label))` | `A((Circle))` |
| Diamond | `{label}` | `A{Decision}` |
| Cylinder | `[(label)]` | `DB[(Database)]` |
| Stadium | `([label])` | `A([Stadium])` |
| Hexagon | `{{label}}` | `A{{Hexagon}}` |
| Arrow | `-->` | `A --> B` |
| Line | `---` | `A --- B` |
| Dotted | `-.->` | `A -.-> B` |
| Thick | `==>` | `A ==> B` |
| Label | `-->\|text\|` | `A -->\|yes\| B` |

### Mermaid State Diagrams

| Feature | Syntax | Example |
|---------|--------|---------|
| State | `StateName` | `Idle` |
| Start | `[*]` | `[*] --> Idle` |
| End | `[*]` | `Done --> [*]` |
| Transition | `-->` | `Idle --> Running` |
| Label | `: text` | `Idle --> Running: start` |
| Description | `state "desc" as ID` | `state "Waiting" as Wait` |

### D2 Diagrams

| Feature | Syntax | Example |
|---------|--------|---------|
| Shape | `id` or `id: label` | `server: Web Server` |
| Arrow | `->` | `A -> B` |
| Reverse | `<-` | `A <- B` |
| Bidirectional | `<->` | `A <-> B` |
| Line | `--` | `A -- B` |
| Edge label | `: text` | `A -> B: request` |
| Shape type | `.shape: type` | `db.shape: cylinder` |
| Container | `{ }` | `backend { api }` |

## Development

```bash
git clone https://github.com/decisiongraph/graphs-tui.git
cd graphs-tui
cargo test
```

## License

AGPL-3.0-or-later

## Inspiration

Inspired by [tariqshams/mermaidtui](https://github.com/tariqshams/mermaidtui)
