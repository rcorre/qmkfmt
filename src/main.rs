use core::str;
use prettytable::Table;
use std::io::Read;
use streaming_iterator::StreamingIterator;

fn main() {
    let mut text = String::new();
    std::io::stdin().read_to_string(&mut text).unwrap();

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
    let args_idx = query.capture_index_for_name("args").unwrap();
    let call_idx = query.capture_index_for_name("call").unwrap();

    let lines: Vec<_> = text.lines().collect();
    let mut last_byte = 0;
    let mut qc = tree_sitter::QueryCursor::new();
    let mut it = qc.matches(&query, tree.root_node(), text.as_bytes());
    while let Some(m) = it.next() {
        let name = m.nodes_for_capture_index(id_idx).next().unwrap();
        let (indent, _) = lines[name.start_position().row]
            .split_once(|c: char| !c.is_whitespace())
            .unwrap();
        let name = name
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

        let keys: Vec<_> = m
            .nodes_for_capture_index(args_idx)
            .next()
            .unwrap()
            .named_children(&mut qc)
            .collect();

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
        let mut rows = vec![vec![]; row_count];
        for (i, key) in keys.iter().enumerate() {
            let row = key.start_position().row - min_row;
            rows[row]
                .push(node_to_text(&text, &key) + if i == (keys.len() - 1) { "" } else { "," });
        }

        let column_count = rows
            .iter()
            .map(|row| row.len())
            .max()
            .expect("Row has 0 columns");

        for row in rows {
            let fill = column_count - row.len();
            table.add_row(
                std::iter::repeat_n("", fill / 2)
                    .chain(row.iter().map(|s| s.as_str()))
                    .chain(std::iter::repeat_n("", fill / 2))
                    .into(),
            );
        }

        // Indent each line of the table
        let table = table
            .to_string()
            .lines()
            .map(|line| line.trim_end())
            .map(|line| format!("{indent}{indent}{line}"))
            .collect::<Vec<_>>()
            .join("\n");
        print!("{name}(\n{table}\n{indent})");
    }

    let rest = &text.as_bytes()[last_byte..];
    let rest = str::from_utf8(rest).expect("Text is not utf-8");
    print!("{rest}");
}

fn node_to_text(text: &str, node: &tree_sitter::Node) -> String {
    node.utf8_text(text.as_bytes())
        .expect("Failed to get text from node")
        .to_string()
}

#[cfg(test)]
mod tests {
    use insta_cmd::{assert_cmd_snapshot, get_cargo_bin};
    use std::process::Command;

    #[test]
    fn test_fmt() {
        let mut cmd = Command::new(get_cargo_bin(env!("CARGO_PKG_NAME")));
        let keymap = std::fs::read_to_string("testdata/keymap.c").unwrap();
        assert_cmd_snapshot!(cmd.pass_stdin(keymap));
    }
}
