//! flowmaid — CLI on top of the flowmaid diagram engine library.
//!
//! Pipeline: .mmd text  ->  parser  ->  layout  ->  SVG.

use flowmaid::{fold, parser, render, Document};
use std::env;
use std::fs;
use std::io::{self, IsTerminal, Read};
use std::process;

fn print_help() {
    println!(
        "flowmaid — a small flowchart diagram engine (Mermaid-like syntax)\n\n\
         Usage:\n\
         \x20 flowmaid <input.mmd> [-o output.svg]\n\
         \x20 cat diagram.mmd | flowmaid > out.svg\n\n\
         Options:\n\
         \x20 -o, --output <file>   write SVG to a file (default: stdout)\n\
         \x20 --compact <px>        fold a long linear chain to fit <px> along\n\
         \x20                       the flow axis (serpentine layout)\n\
         \x20 -h, --help            show this help\n\n\
         Syntax example:\n\
         \x20 flowchart TD\n\
         \x20 A([Start]) --> B{{Valid?}}\n\
         \x20 B -->|yes| C[Process]\n\
         \x20 B -.->|no| D((Done))"
    );
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut input: Option<String> = None;
    let mut output: Option<String> = None;
    let mut compact: Option<f64> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output" => {
                i += 1;
                match args.get(i) {
                    Some(p) => output = Some(p.clone()),
                    None => {
                        eprintln!("option -o requires a file name");
                        process::exit(2);
                    }
                }
            }
            "--compact" => {
                i += 1;
                match args.get(i).and_then(|v| v.parse::<f64>().ok()) {
                    Some(px) if px > 0.0 => compact = Some(px),
                    _ => {
                        eprintln!("option --compact requires a pixel budget, e.g. --compact 600");
                        process::exit(2);
                    }
                }
            }
            "-h" | "--help" => {
                print_help();
                return;
            }
            other if other.starts_with('-') => {
                eprintln!("unknown option: '{}' (see --help)", other);
                process::exit(2);
            }
            other => {
                if input.is_some() {
                    eprintln!("only one input file is supported, extra: '{}'", other);
                    process::exit(2);
                }
                input = Some(other.to_string());
            }
        }
        i += 1;
    }

    let source = match &input {
        Some(path) => fs::read_to_string(path).unwrap_or_else(|e| {
            eprintln!("failed to read '{}': {}", path, e);
            process::exit(1);
        }),
        None => {
            // No argument and no pipe: don't silently wait on stdin,
            // show the help instead.
            if io::stdin().is_terminal() {
                print_help();
                return;
            }
            let mut buf = String::new();
            if io::stdin().read_to_string(&mut buf).is_err() {
                eprintln!("failed to read stdin");
                process::exit(1);
            }
            buf
        }
    };

    let doc = match parser::parse_document(&source) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("parse error — {}", e);
            process::exit(1);
        }
    };
    let svg = match &doc {
        Document::Flowchart(g) if g.nodes.is_empty() => {
            eprintln!("empty diagram: no nodes defined");
            process::exit(1);
        }
        Document::Er(d) if d.entities.is_empty() => {
            eprintln!("empty diagram: no entities defined");
            process::exit(1);
        }
        Document::Class(d) if d.classes.is_empty() => {
            eprintln!("empty diagram: no classes defined");
            process::exit(1);
        }
        Document::Sequence(d) if d.participants.is_empty() => {
            eprintln!("empty diagram: no participants defined");
            process::exit(1);
        }
        Document::Pie(d) if d.slices.is_empty() => {
            eprintln!("empty diagram: no data rows");
            process::exit(1);
        }
        Document::State(g) if g.nodes.is_empty() => {
            eprintln!("empty diagram: no states defined");
            process::exit(1);
        }
        Document::Mindmap(d) if d.nodes.is_empty() => {
            eprintln!("empty diagram: no mindmap nodes");
            process::exit(1);
        }
        Document::Journey(d) if d.sections.iter().all(|s| s.tasks.is_empty()) => {
            eprintln!("empty diagram: no journey tasks");
            process::exit(1);
        }
        Document::Flowchart(g) | Document::State(g) => match compact {
            Some(px) => fold::render_compact(g, &fold::CompactOptions::for_extent(px)),
            None => render::render(g),
        },
        Document::Er(d) => render::render_er(d),
        Document::Class(d) => render::render_class(d),
        Document::Sequence(d) => render::render_seq(d),
        Document::Pie(d) => render::render_pie(d),
        Document::Mindmap(d) => render::render_mindmap(d),
        Document::Journey(d) => render::render_journey(d),
    };

    match &output {
        Some(path) => {
            if let Err(e) = fs::write(path, svg) {
                eprintln!("failed to write '{}': {}", path, e);
                process::exit(1);
            }
            eprintln!("saved: {}", path);
        }
        None => print!("{}", svg),
    }
}
