use std::{
    fs::{self, read_to_string},
    path::{Path, PathBuf},
    process::Command,
};

use cargo_metadata::MetadataCommand;
use rustdoc_types::Crate;
use rustfix::{LinePosition, LineRange, Replacement, Snippet, Solution, Suggestion};

fn main() {
    let exposed_crates = find_exposed_deps();
    let exports_to_add = filter_already_exported(exposed_crates);

    add_exports(exports_to_add);
}

fn find_exposed_deps() -> Vec<String> {
    let rustdoc_output = get_rustdoc_output();
    find_foreign_items(rustdoc_output)
}

fn get_rustdoc_output() -> Crate {
    let metadata = MetadataCommand::new().exec().unwrap();

    let crate_name = metadata.root_package().unwrap().name.replace("-", "_");

    Command::new("cargo")
        .args([
            "+nightly",
            "rustdoc",
            "--",
            "-Zunstable-options",
            "--output-format=json",
        ])
        .status()
        .unwrap();

    let output_file_name = format!("{crate_name}.json");
    let mut output_file_path = PathBuf::from("target/doc/");
    output_file_path.push(output_file_name);

    let content = fs::read_to_string(output_file_path).unwrap();

    serde_json::from_str(content.as_str()).unwrap()
}

fn find_foreign_items(krate: Crate) -> Vec<String> {
    krate
        .external_crates
        .values()
        .map(|external_crate| external_crate.name.clone())
        .collect()
}

fn filter_already_exported(deps: Vec<String>) -> Vec<String> {
    deps
}

fn add_exports(deps: Vec<String>) {
    let import_suggestion = ImportSuggestion::from_deps(deps).to_rust_import();

    let file_to_read = Path::new("src/main.rs");
    let code = read_to_string(file_to_read).unwrap();
    // Don't ask questions
    let code = format!(" {code}");

    let replacement = Replacement {
        snippet: Snippet {
            file_name: String::new(),
            line_range: LineRange {
                start: LinePosition { line: 1, column: 1 },
                end: LinePosition { line: 1, column: 1 },
            },
            range: 0..0,
            text: (String::new(), String::new(), "use std::{".to_string()),
        },
        replacement: import_suggestion,
    };

    let solution = Solution {
        message: String::new(),
        replacements: vec![replacement],
    };

    let suggestion = Suggestion {
        message: String::new(),
        snippets: Vec::new(),
        solutions: vec![solution],
    };

    let suggestions = vec![suggestion];

    let code = rustfix::apply_suggestions(code.as_str(), suggestions.as_slice()).unwrap();

    println!("{code}");
}

struct ImportSuggestion {
    deps: Vec<String>,
}

impl ImportSuggestion {
    fn from_deps(mut deps: Vec<String>) -> ImportSuggestion {
        deps.sort();
        ImportSuggestion { deps }
    }

    fn to_rust_import(&self) -> String {
        let mut buff = String::new();

        buff.push_str("pub mod deps {\n");

        self.deps.iter().for_each(|dep_name| {
            buff.push_str("    pub use ");
            buff.push_str(dep_name.as_str());
            buff.push_str(";\n");
        });

        buff.push_str("}\n");

        buff
    }
}
