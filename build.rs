use std::path::PathBuf;

fn main() {
    let top_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
        .canonicalize()
        .unwrap();

    protobuf_codegen::Codegen::new()
        .cargo_out_dir("generated")
        .inputs([top_dir.join("janitor/config.proto")])
        .include(top_dir)
        .run_from_script();
}
