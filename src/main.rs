use core::str;
use std::{
    fs::File,
    io::{Read, Write},
    path::PathBuf,
    process::Stdio,
};
use streaming_iterator::StreamingIterator;

use clap::Parser;
use tree_sitter::Node;

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Path to keymap.c to format. If omitted, reads stdin and writes to stdout.
    path: Option<PathBuf>,

    /// Number of spaces to insert between sides of the keyboard.
    #[arg(long)]
    split_spaces: Option<usize>,

    /// Disable running clang-format.
    #[arg(long)]
    no_clang_format: bool,

    /// Path to clang-format, or empty to disable clang-format.
    #[arg(long, default_value = "clang-format")]
    clang_format: PathBuf,
}

fn clang_format(cli: &Cli, text: &str) -> String {
    if cli.no_clang_format {
        log::debug!("clang-format disabled");
        return text.to_string();
    }
    let mut cmd = std::process::Command::new(&cli.clang_format);
    cmd.stdin(Stdio::piped()).stdout(Stdio::piped());
    let mut cmd = match cmd.spawn() {
        Ok(cmd) => cmd,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            log::debug!("clang-format not installed");
            return text.to_string();
        }
        Err(err) => panic!("Failed to exec clang-format: {err:?}"),
    };

    log::trace!("Running clang-format on text: {text:#?}");
    cmd.stdin
        .take()
        .unwrap()
        .write_all(text.as_bytes())
        .expect("Failed to write to clang-format");
    let output = cmd
        .wait_with_output()
        .expect("Failed to wait for clang-format");
    if !output.status.success() {
        panic!("clang-format exited with code: {:?}", output.status.code());
    }
    log::debug!("clang-format succeeded");

    let out = String::from_utf8(output.stdout).expect("clang-format output is not utf-8");
    log::trace!("clang-format output: {out:#?}");

    // Add a newline, to mantain spacing from the start of keymaps.
    out + "\n"
}

fn main() {
    env_logger::init();
    let cli = Cli::parse();
    log::debug!("Parsed args: {cli:?}");
    match cli.path {
        Some(ref path) => {
            log::info!("Formatting path: {path:?}");
            let text = std::fs::read_to_string(path).expect("Failed to read");
            let mut output = File::create(path).expect("Failed to open for writing");
            format(&text, &mut output, &cli);
        }
        None => {
            log::info!("Formatting stdin");
            let mut text = String::new();
            std::io::stdin()
                .read_to_string(&mut text)
                .expect("Failed to read stdin");
            format(&text, &mut std::io::stdout(), &cli);
        }
    };
}

fn key_rows(text: &str, keys: &[Node<'_>]) -> Vec<Vec<String>> {
    // Group keys by row
    let min_row = keys
        .iter()
        .map(|node| node.start_position().row)
        .min()
        .unwrap();
    let max_row = keys
        .iter()
        .map(|node| node.start_position().row)
        .max()
        .unwrap();

    let row_count = max_row - min_row + 1;
    log::debug!("Row count: {row_count}");
    let mut rows = vec![vec![]; row_count];
    for (i, key) in keys.iter().enumerate() {
        let row = key.start_position().row - min_row;
        rows[row].push(node_to_text(text, key) + if i == (keys.len() - 1) { "" } else { "," });
    }

    rows
}

fn format(text: &str, output: &mut impl Write, cli: &Cli) {
    let language = tree_sitter_c::LANGUAGE.into();
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&language)
        .expect("Error loading C parser");

    let query = tree_sitter::Query::new(
        &language,
        r#"(call_expression (identifier) @id (#match? @id "^LAYOUT") (argument_list) @args) @call"#,
    )
    .unwrap();
    let id_idx = query.capture_index_for_name("id").unwrap();
    let args_idx = query.capture_index_for_name("args").unwrap();
    let call_idx = query.capture_index_for_name("call").unwrap();

    // First, extract all the layouts before clang-format
    let tree = parser.parse(text, None).unwrap();
    let mut layouts = vec![];
    let mut qc = tree_sitter::QueryCursor::new();
    let mut it = qc.matches(&query, tree.root_node(), text.as_bytes());
    while let Some(m) = it.next() {
        let args_node = m.nodes_for_capture_index(args_idx).next().unwrap();
        let mut qc = args_node.walk();
        let keys: Vec<_> = args_node.named_children(&mut qc).collect();
        let rows = key_rows(text, &keys);
        layouts.push(rows);
    }
    layouts.reverse(); // so we can pop

    // Run clang-format on the document
    let text = &clang_format(cli, text);

    // Parse again, post-clang-format
    let tree = parser.parse(text, None).unwrap();

    let lines: Vec<_> = text.lines().collect();
    let mut qc = tree_sitter::QueryCursor::new();
    let mut it = qc.matches(&query, tree.root_node(), text.as_bytes());
    let mut last_byte = 0;
    while let Some(m) = it.next() {
        let name = m.nodes_for_capture_index(id_idx).next().unwrap();
        let (indent, _) = lines[name.start_position().row]
            .split_once(|c: char| !c.is_whitespace())
            .unwrap();
        let indent = if indent.is_empty() { "    " } else { indent };
        let name = node_to_text(text, &name);
        log::trace!("Parsed call_expression: {name}");
        log::trace!("Printing prefix to layout: {name}");

        // Print everything before the call expression
        let args_node = m.nodes_for_capture_index(args_idx).next().unwrap();
        let prefix = &text.as_bytes()[last_byte..args_node.start_byte()];
        let prefix = str::from_utf8(prefix).expect("Text is not utf-8");
        write!(output, "{prefix}").expect("Failed to write layout prefix");

        // Print the formatted key list inside parens
        let mut rows = layouts.pop().unwrap();
        let column_count = rows.iter().map(|r| r.len()).max().expect("No rows");

        log::trace!("Rows: {rows:?}");

        // Pad shorter rows on the left
        for row in rows.iter_mut() {
            let fill = column_count - row.len();
            for _ in 0..fill / 2 {
                row.insert(0, "".into())
            }
        }

        log::trace!("Padded rows: {rows:?}");

        let column_sizes: Vec<_> = (0..column_count)
            .map(|i| {
                rows.iter()
                    .map(|row| row.get(i).map(String::len).unwrap_or(0))
                    .max()
                    .unwrap_or(0)
            })
            .collect();

        log::trace!("Column sizes: {column_sizes:?}");

        writeln!(output, "(").unwrap();
        for row in rows {
            log::trace!("Writing row: {row:?}");
            write!(output, "{indent}{indent}").unwrap();
            for (i, col) in row.iter().enumerate() {
                if i == column_count / 2 {
                    write!(output, "{}", " ".repeat(cli.split_spaces.unwrap_or(0))).unwrap();
                }
                let separator = if i + 1 < row.len() { " " } else { "" };
                let width = if i + 1 < row.len() {
                    column_sizes[i]
                } else {
                    0
                };
                write!(output, "{col:width$}{separator}").unwrap();
            }
            writeln!(output).unwrap();
        }
        write!(output, "{indent})").unwrap();

        let call_node = m.nodes_for_capture_index(call_idx).next().unwrap();
        last_byte = call_node.end_byte();
    }

    log::debug!("Writing suffix");

    let rest = &text.as_bytes()[last_byte..];
    let rest = str::from_utf8(rest).expect("Text is not utf-8");
    write!(output, "{rest}").unwrap();

    log::info!("Formatting complete!");
}

fn node_to_text(text: &str, node: &tree_sitter::Node) -> String {
    node.utf8_text(text.as_bytes())
        .expect("Failed to get text from node")
        .to_string()
}
