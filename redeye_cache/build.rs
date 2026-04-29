// redeye_cache/build.rs
//
// Compiles the shared `semantic_cache.proto` into Rust code.
// Builds the SERVER stubs only (gateway builds the client stubs in its own build.rs).

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protoc_path = protoc_bin_vendored::protoc_bin_path()
        .map_err(|e| format!("Failed to locate vendored protoc: {}", e))?;

    std::env::set_var("PROTOC", protoc_path);

    tonic_build::configure()
        .build_server(true) // Cache service only needs the server stub.
        .build_client(false)
        .compile_protos(&["../proto/semantic_cache.proto"], &["../proto"])?;

    println!("cargo:rerun-if-changed=../proto/semantic_cache.proto");

    Ok(())
}
