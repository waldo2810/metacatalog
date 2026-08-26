// Copyright 2026 The MetaCatalog Authors. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

//! The `mc` binary. Command-line parsing proper arrives with INFRA-02; for now
//! this is a thin front end over the library that dumps the parse tree of the
//! YAML files it is given, which is what INFRA-01's verification steps need.

use std::path::Path;
use std::process::ExitCode;

use mc::yaml::{self, Node, Value};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        println!("MetaCatalog");
        return ExitCode::SUCCESS;
    }
    for arg in &args {
        match yaml::parse_file(Path::new(arg)) {
            Ok(node) => {
                let mut out = String::new();
                dump(&node, 0, &mut out);
                print!("{arg}\n{out}");
            }
            Err(e) => {
                eprintln!("{e}");
                return ExitCode::FAILURE;
            }
        }
    }
    ExitCode::SUCCESS
}

/// Render the tree with each node's source line in the left column.
fn dump(node: &Node, depth: usize, out: &mut String) {
    let pad = "  ".repeat(depth);
    match &node.value {
        Value::Map(entries) => {
            for (key, value) in entries {
                match value.value {
                    Value::Map(_) | Value::Seq(_) => {
                        out.push_str(&format!("{:>5}  {pad}{}:\n", key.line, key.name));
                        dump(value, depth + 1, out);
                    }
                    _ => out.push_str(&format!(
                        "{:>5}  {pad}{}: {}\n",
                        key.line,
                        key.name,
                        scalar(value)
                    )),
                }
            }
        }
        Value::Seq(items) => {
            for item in items {
                match item.value {
                    Value::Map(_) | Value::Seq(_) => {
                        out.push_str(&format!("{:>5}  {pad}-\n", item.line));
                        dump(item, depth + 1, out);
                    }
                    _ => out.push_str(&format!("{:>5}  {pad}- {}\n", item.line, scalar(item))),
                }
            }
        }
        _ => out.push_str(&format!("{:>5}  {pad}{}\n", node.line, scalar(node))),
    }
}

fn scalar(node: &Node) -> String {
    let text = match &node.value {
        Value::Null => "~".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Int(i) => i.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Str(s) => format!("{s:?}"),
        Value::Seq(_) | Value::Map(_) => String::new(),
    };
    format!("{text} ({})", node.type_name())
}
