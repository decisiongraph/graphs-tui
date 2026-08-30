use graphs_tui::{RenderOptions, render_diagram};
use std::io::{self};
use std::process::ExitCode;

fn main() -> ExitCode {
    let stdin = io::stdin();
    let mut content = String::new();

    while stdin.read_line(&mut content).unwrap() != 0 { }

    match render_diagram(&content, RenderOptions::default()) {
        Ok(graph) => {
            println!("{}", graph.output);
            ExitCode::from(0)
        }
        Err(error) => {
            eprintln!("{}", error);
            ExitCode::from(1)
        }
    }
}
