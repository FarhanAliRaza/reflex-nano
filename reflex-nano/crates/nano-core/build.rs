//! Minify the fixed DOM adapter once when building the Rust framework.
use std::{env, fs, path::PathBuf};
fn main() {
    println!("cargo:rerun-if-changed=frontend-dist");
    println!("cargo:rerun-if-changed=src/client.js");
    let source = fs::read("src/client.js").expect("browser adapter");
    let session = minify_js::Session::new();
    let mut output = Vec::new();
    // Use the syntax-aware emitter. The optional minify-js control-flow rewrite
    // panics on valid nested return/throw branches in this adapter.
    let parsed = parse_js::parse(&session, &source, minify_js::TopLevelMode::Global)
        .expect("browser adapter must be valid JavaScript");
    minify_js::emit(parsed, &mut output);
    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("client.min.js"),
        output,
    )
    .expect("write minified browser adapter");
    let directory = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("frontend-dist");
    let mut files: Vec<_> = fs::read_dir(&directory)
        .expect("Run npm run build:frontend first")
        .map(|p| p.unwrap().path())
        .filter(|p| matches!(p.extension().and_then(|v| v.to_str()), Some("js" | "css")))
        .collect();
    files.sort();
    assert!(
        directory.join("react.js").exists(),
        "Missing React frontend: npm run build:frontend"
    );
    let mut index = String::from("pub const REACT_FILES: &[(&str, &str)] = &[\n");
    for path in files {
        index.push_str(&format!(
            "({:?}, include_str!({:?})),\n",
            path.file_name().unwrap().to_str().unwrap(),
            path.to_str().unwrap()
        ));
    }
    index.push_str("];\n");
    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("react_assets.rs"),
        index,
    )
    .unwrap();
}
