use core::str;
use prettytable::Table;
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
        "(call_expression (identifier) @id (argument_list) @args) @call",
    )
    .unwrap();
    let id_idx = query.capture_index_for_name("id").unwrap();
    assert_eq!(id_idx, 0);
    let args_idx = query.capture_index_for_name("args").unwrap();
    assert_eq!(args_idx, 1);
    let call_idx = query.capture_index_for_name("call").unwrap();
    assert_eq!(call_idx, 2);
    
    
    let mut qc = tree_sitter::QueryCursor::new();

    let column_count = 12;

    let mut last_byte = 0;
    let mut it = qc.matches(&query, tree.root_node(), text.as_bytes());
    while let Some(m) = it.next() {
        let indent = ""; // TODO: determine from initializer_pair
        // Ensure this is a layout call, and if so, extract the name (e.g. LAYOUT_split_3x6_3)
        let name = m.nodes_for_capture_index(id_idx).next().unwrap()
            .utf8_text(text.as_bytes())
            .expect("Failed to get text from node");
        if !name.starts_with("LAYOUT") {
            continue;
        }

        // Print everything before the call expression
        let call_node = m.nodes_for_capture_index(call_idx).next().unwrap();
        let prefix = &text.as_bytes()[last_byte..call_node.start_byte()];
        let prefix = str::from_utf8(prefix).expect("Text is not utf-8");
        last_byte = call_node.end_byte();
        print!("{prefix}");

        // Print the formatted key list inside parens
        let mut table = Table::new();
        table.set_format(*prettytable::format::consts::FORMAT_CLEAN);
        let node = m.captures[1].node;
        let mut qc = node.walk();

        let keys: Vec<_> = m.nodes_for_capture_index(args_idx).next().unwrap()
            .named_children(&mut qc)
            .map(|node| node_to_text(&text, &node) + ",")
            .collect();

        for row in keys.chunks(column_count) {
            let fill = column_count - row.len();
            table.add_row(
                std::iter::repeat_n("", fill / 2)
                    .chain(row.iter().map(|s| s.as_str()))
                    .chain(std::iter::repeat_n("", fill / 2))
                    .into(),
            );
        }

        print!("{name}(\n{table}{indent})")
    }

    let rest = &text.as_bytes()[last_byte..];
    let rest = str::from_utf8(rest).expect("Text is not utf-8");
    println!("{rest}");
}

fn node_to_text(text: &str, node: &tree_sitter::Node) -> String {
    node.utf8_text(text.as_bytes())
        .expect("Failed to get text from node")
        .to_string()
}
