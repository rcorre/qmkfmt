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

/// Write a grid of rows with aligned columns, centering shorter rows.
fn write_grid(
    output: &mut impl Write,
    rows: &mut [Vec<String>],
    indent: &str,
    split_spaces: usize,
) {
    let column_count = rows.iter().map(|r| r.len()).max().unwrap_or(0);

    // Pad shorter rows on the left
    for row in rows.iter_mut() {
        let fill = column_count - row.len();
        for _ in 0..fill / 2 {
            row.insert(0, "".into())
        }
    }

    let column_sizes: Vec<_> = (0..column_count)
        .map(|i| {
            rows.iter()
                .map(|row| row.get(i).map(String::len).unwrap_or(0))
                .max()
                .unwrap_or(0)
        })
        .collect();

    for row in rows.iter() {
        write!(output, "{indent}{indent}").unwrap();
        for (i, col) in row.iter().enumerate() {
            if i == column_count / 2 {
                write!(output, "{}", " ".repeat(split_spaces)).unwrap();
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
}

/// Build ledmap rows from flat tuples using the given row structure (items per row).
fn build_ledmap_rows(text: &str, tuples: &[Node<'_>], row_structure: &[usize]) -> Vec<Vec<String>> {
    let mut rows = vec![];
    let mut idx = 0;
    for &cols_in_row in row_structure {
        let mut row = vec![];
        for _ in 0..cols_in_row {
            if idx < tuples.len() {
                let s = node_to_text(text, &tuples[idx]);
                let suffix = if idx == tuples.len() - 1 { "" } else { "," };
                row.push(s + suffix);
                idx += 1;
            }
        }
        rows.push(row);
    }
    rows
}

fn format(text: &str, output: &mut impl Write, cli: &Cli) {
    let language = tree_sitter_c::LANGUAGE.into();
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&language)
        .expect("Error loading C parser");

    // Two top-level patterns returned in document order by QueryCursor::matches():
    //   Pattern 0: LAYOUT call expressions (structured match)
    //   Pattern 1: "ledmap" identifier (simple match — tree-sitter-c mis-parses
    //              PROGMEM as the declarator name, so "ledmap" ends up in an ERROR
    //              node and can't be matched structurally)
    let query = tree_sitter::Query::new(
        &language,
        r#"
            (call_expression
                (identifier) @id (#match? @id "^LAYOUT")
                (argument_list) @args) @call

            ((identifier) @id (#eq? @id "ledmap"))
        "#,
    )
    .unwrap();
    let id_idx = query.capture_index_for_name("id").unwrap();
    let args_idx = query.capture_index_for_name("args").unwrap();
    let call_idx = query.capture_index_for_name("call").unwrap();

    // First pass: extract LAYOUT row structures before clang-format
    let tree = parser.parse(text, None).unwrap();
    let mut layouts = vec![];
    let mut layout_structures: Vec<Vec<usize>> = vec![];
    let mut qc = tree_sitter::QueryCursor::new();
    let mut it = qc.matches(&query, tree.root_node(), text.as_bytes());
    while let Some(m) = it.next() {
        if m.pattern_index != 0 {
            continue;
        }
        let args_node = m.nodes_for_capture_index(args_idx).next().unwrap();
        let mut wc = args_node.walk();
        let keys: Vec<_> = args_node.named_children(&mut wc).collect();
        let rows = key_rows(text, &keys);
        layout_structures.push(rows.iter().map(|row| row.len()).collect());
        layouts.push(rows);
    }

    layouts.reverse(); // so we can pop

    // Run clang-format on the document
    let text = &clang_format(cli, text);

    // Second pass: format both LAYOUTs and ledmaps in document order
    let tree = parser.parse(text, None).unwrap();
    let lines: Vec<_> = text.lines().collect();
    let mut qc = tree_sitter::QueryCursor::new();
    let mut it = qc.matches(&query, tree.root_node(), text.as_bytes());
    let mut last_byte = 0;
    while let Some(m) = it.next() {
        let name_node = m.nodes_for_capture_index(id_idx).next().unwrap();

        if m.pattern_index == 0 {
            // LAYOUT formatting
            let call_node = m.nodes_for_capture_index(call_idx).next().unwrap();
            let args_node = m.nodes_for_capture_index(args_idx).next().unwrap();

            let (indent, _) = lines[name_node.start_position().row]
                .split_once(|c: char| !c.is_whitespace())
                .unwrap_or(("    ", ""));
            let indent = if indent.is_empty() { "    " } else { indent };

            let name = node_to_text(text, &name_node);
            log::trace!("Parsed call_expression: {name}");

            let prefix = &text.as_bytes()[last_byte..args_node.start_byte()];
            write!(output, "{}", str::from_utf8(prefix).unwrap()).unwrap();

            let mut rows = layouts.pop().unwrap();
            writeln!(output, "(").unwrap();
            write_grid(output, &mut rows, indent, cli.split_spaces.unwrap_or(0));
            write!(output, "{indent})").unwrap();

            last_byte = call_node.end_byte();
        } else {
            // ledmap: walk up from the identifier to find init_declarator + initializer_list
            let decl = {
                let mut current = name_node.parent();
                loop {
                    match current {
                        Some(node) if node.kind() == "init_declarator" => break Some(node),
                        Some(node) => current = node.parent(),
                        None => break None,
                    }
                }
            };
            let decl = match decl {
                Some(d) => d,
                None => continue,
            };
            let mut cursor = decl.walk();
            let init_list = match decl
                .children(&mut cursor)
                .find(|c| c.kind() == "initializer_list")
            {
                Some(il) => il,
                None => continue,
            };

            // Skip if this "ledmap" is a reference inside an expression (e.g.
            // `&ledmap[layer][i][0]`), not the actual declarator.  The identifier
            // must appear before the initializer_list (i.e. on the left of `=`).
            if name_node.start_byte() >= init_list.start_byte() {
                continue;
            }

            log::debug!("Found ledmap, formatting it");

            let (indent, _) = lines[name_node.start_position().row]
                .split_once(|c: char| !c.is_whitespace())
                .unwrap_or(("    ", ""));
            let indent = if indent.is_empty() { "    " } else { indent };

            let prefix = &text.as_bytes()[last_byte..init_list.start_byte()];
            write!(output, "{}", str::from_utf8(prefix).unwrap()).unwrap();

            writeln!(output, "{{").unwrap();

            let mut layer_cursor = init_list.walk();
            let layers: Vec<_> = init_list.named_children(&mut layer_cursor).collect();

            for (layer_i, layer) in layers.iter().enumerate() {
                let mut parts_cursor = layer.walk();
                let mut layer_designator = None;
                let mut layer_init = None;

                for part in layer.children(&mut parts_cursor) {
                    if part.kind() == "subscript_designator" {
                        layer_designator = Some(node_to_text(text, &part));
                    } else if part.kind() == "initializer_list" {
                        layer_init = Some(part);
                    }
                }

                let (designator, inner_init) = match (layer_designator, layer_init) {
                    (Some(d), Some(i)) => (d, i),
                    _ => continue,
                };

                let mut tuple_cursor = inner_init.walk();
                let tuples: Vec<_> = inner_init.named_children(&mut tuple_cursor).collect();

                let layer_num: Option<usize> = designator
                    .trim_matches(|c| c == '[' || c == ']')
                    .parse()
                    .ok();

                let row_structure = layer_num.and_then(|n| layout_structures.get(n));
                let matches_layout = row_structure
                    .map(|rs| rs.iter().sum::<usize>() == tuples.len())
                    .unwrap_or(false);

                let mut rows = if matches_layout {
                    build_ledmap_rows(text, &tuples, row_structure.unwrap())
                } else {
                    if let Some(rs) = row_structure {
                        log::warn!(
                            "Ledmap layer {} has {} tuples but layout has {} keys, using fixed formatting",
                            designator,
                            tuples.len(),
                            rs.iter().sum::<usize>()
                        );
                    }
                    let cols = layout_structures
                        .first()
                        .and_then(|rs| rs.iter().max().copied())
                        .unwrap_or(6);
                    build_ledmap_rows(text, &tuples, &vec![cols; tuples.len().div_ceil(cols)])
                };

                write!(output, "{indent}{designator} = {{").unwrap();
                writeln!(output).unwrap();
                write_grid(output, &mut rows, indent, cli.split_spaces.unwrap_or(0));
                write!(output, "{indent}}}").unwrap();
                if layer_i < layers.len() - 1 {
                    writeln!(output, ",").unwrap();
                    writeln!(output).unwrap();
                } else {
                    writeln!(output).unwrap();
                }
            }

            write!(output, "}};").unwrap();

            // Advance past the init_declarator and its trailing semicolon
            let after_decl = &text.as_bytes()[decl.end_byte()..];
            let skip = after_decl
                .iter()
                .position(|&b| b == b';')
                .map(|p| p + 1)
                .unwrap_or(0);
            last_byte = decl.end_byte() + skip;
        }
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
