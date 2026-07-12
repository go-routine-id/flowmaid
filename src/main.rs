//! flowmaid — CLI di atas pustaka mesin diagram flowmaid.
//!
//! Pipeline: teks .mmd  ->  parser  ->  layout  ->  SVG.

use flowmaid::{parser, render};
use std::env;
use std::fs;
use std::io::{self, IsTerminal, Read};
use std::process;

fn print_help() {
    println!(
        "flowmaid — mesin diagram flowchart mini (sintaks ala Mermaid)\n\n\
         Pemakaian:\n\
         \x20 flowmaid <input.mmd> [-o output.svg]\n\
         \x20 cat diagram.mmd | flowmaid > out.svg\n\n\
         Opsi:\n\
         \x20 -o, --output <file>   tulis SVG ke file (default: stdout)\n\
         \x20 -h, --help            tampilkan bantuan ini\n\n\
         Contoh sintaks:\n\
         \x20 flowchart TD\n\
         \x20 A([Mulai]) --> B{{Valid?}}\n\
         \x20 B -->|ya| C[Proses]\n\
         \x20 B -.->|tidak| D((Selesai))"
    );
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut input: Option<String> = None;
    let mut output: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output" => {
                i += 1;
                match args.get(i) {
                    Some(p) => output = Some(p.clone()),
                    None => {
                        eprintln!("opsi -o membutuhkan nama file");
                        process::exit(2);
                    }
                }
            }
            "-h" | "--help" => {
                print_help();
                return;
            }
            other if other.starts_with('-') => {
                eprintln!("opsi tidak dikenal: '{}' (lihat --help)", other);
                process::exit(2);
            }
            other => {
                if input.is_some() {
                    eprintln!("hanya satu file input yang didukung, kelebihan: '{}'", other);
                    process::exit(2);
                }
                input = Some(other.to_string());
            }
        }
        i += 1;
    }

    let source = match &input {
        Some(path) => fs::read_to_string(path).unwrap_or_else(|e| {
            eprintln!("gagal membaca '{}': {}", path, e);
            process::exit(1);
        }),
        None => {
            // Tanpa argumen dan tanpa pipe: jangan diam menunggu stdin,
            // tampilkan bantuan.
            if io::stdin().is_terminal() {
                print_help();
                return;
            }
            let mut buf = String::new();
            if io::stdin().read_to_string(&mut buf).is_err() {
                eprintln!("gagal membaca stdin");
                process::exit(1);
            }
            buf
        }
    };

    let graph = match parser::parse(&source) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("error parsing — {}", e);
            process::exit(1);
        }
    };
    if graph.nodes.is_empty() {
        eprintln!("diagram kosong: tidak ada node yang terdefinisi");
        process::exit(1);
    }

    let svg = render::render(&graph);

    match &output {
        Some(path) => {
            if let Err(e) = fs::write(path, svg) {
                eprintln!("gagal menulis '{}': {}", path, e);
                process::exit(1);
            }
            eprintln!("tersimpan: {}", path);
        }
        None => print!("{}", svg),
    }
}
