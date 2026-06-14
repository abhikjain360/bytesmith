use std::path::PathBuf;

use crate::{CodeGen, Error};

mod arrays;
mod fields;
mod hooks;
mod layout;
mod len_bounds;
mod primitives;
mod structs;
mod unions;
mod writer;
mod writer_snapshots;

fn generate(dsl: &str) -> String {
    let ast = binparse_dsl_parse::parse_str(dsl).expect("failed to parse DSL");
    CodeGen::generate(&ast).expect("failed to generate code")
}

fn generate_writers(dsl: &str) -> String {
    let ast = binparse_dsl_parse::parse_str(dsl).expect("failed to parse DSL");
    CodeGen::generate_writers(&ast).expect("failed to generate code")
}

fn generate_err(dsl: &str) -> Error {
    let ast = binparse_dsl_parse::parse_str(dsl).expect("failed to parse DSL");
    CodeGen::generate(&ast).expect_err("expected codegen to fail")
}

fn normalized_items(code: &str) -> Vec<String> {
    let file: syn::File = syn::parse_str(code).expect("failed to parse code as Rust");
    let mut items = file
        .items
        .into_iter()
        .map(|item| {
            prettyplease::unparse(&syn::File {
                shebang: None,
                attrs: vec![],
                items: vec![item],
            })
        })
        .collect::<Vec<_>>();
    items.sort();
    items
}

fn snapshot_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/tests/snapshots")
        .join(format!("{name}.txt"))
}

fn assert_generated_eq(dsl: &str, snapshot: &str) {
    let actual = normalized_items(&generate(dsl)).join("\n");
    let path = snapshot_path(snapshot);
    if std::env::var_os("BLESS").is_some() {
        std::fs::write(&path, format!("{actual}\n"))
            .unwrap_or_else(|e| panic!("failed to write snapshot {snapshot}: {e}"));
        return;
    }
    let expected = std::fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!("missing snapshot {snapshot}; rerun with BLESS=1 to regenerate")
    });
    let expected = expected.strip_suffix('\n').unwrap_or(&expected);
    if actual != expected {
        panic!(
            "generated code does not match snapshot {snapshot}; rerun with BLESS=1 to regenerate\n\n--- expected ---\n{expected}\n--- actual ---\n{actual}"
        );
    }
}
