use prettytable::{
    format::{FormatBuilder, TableFormat},
    Table,
};
use std::path::PathBuf;
use streaming_iterator::StreamingIterator;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to keymap.c to format.
    path: PathBuf,
}

fn main() {
    let args = Args::parse();

    let text = std::fs::read_to_string(args.path).expect("Failed to read file");

    let language = tree_sitter_c::LANGUAGE.into();
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&language)
        .expect("Error loading C parser");

    let tree = parser.parse(&text, None).unwrap();
    let query = tree_sitter::Query::new(
        &language,
        "(call_expression (identifier) @id (argument_list) @args)",
    )
    .unwrap();
    let mut qc = tree_sitter::QueryCursor::new();

    let mut it = qc.matches(&query, tree.root_node(), text.as_bytes());
    while let Some(m) = it.next() {
        let node = m.captures[0].node;
        let name = node
            .utf8_text(text.as_bytes())
            .expect("Failed to get text from node");
        if !name.starts_with("LAYOUT") {
            continue;
        }
        eprintln!("{name}");

        let mut table = Table::new();
        table.set_format(*prettytable::format::consts::FORMAT_CLEAN);
        let node = m.captures[1].node;
        let mut qc = node.walk();

        let keys: Vec<_> = m.captures[1]
            .node
            .named_children(&mut qc)
            .map(|node| node_to_text(&text, &node) + ",")
            .collect();

        for row in keys.chunks(12) {
            let fill = 12 - row.len();
            table.add_row(
                std::iter::repeat_n("", fill / 2)
                    .chain(row.iter().map(|s| s.as_str()))
                    .chain(std::iter::repeat_n("", fill / 2))
                    .into(),
            );
        }

        println!("{}", table)
    }
}

fn node_to_text(text: &str, node: &tree_sitter::Node) -> String {
    node.utf8_text(text.as_bytes())
        .expect("Failed to get text from node")
        .to_string()
}
