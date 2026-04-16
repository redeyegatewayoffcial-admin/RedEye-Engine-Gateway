// redeye_gateway/build.rs
//
// Compiles the shared `semantic_cache.proto` into Rust code at build time.
// Uses `protoc-bin-vendored` so no system-level `protoc` installation is needed —
// this keeps CI and developer environments identical on Windows/Linux/macOS.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Point tonic-build at the vendored protoc binary.
    let protoc_path = protoc_bin_vendored::protoc_bin_path()
        .map_err(|e| format!("Failed to locate vendored protoc: {}", e))?;

    std::env::set_var("PROTOC", protoc_path);

    tonic_build::configure()
        // Build server stubs too — needed by integration tests that spin up
        // in-process mock gRPC servers via CacheServiceServer::new(...).
        .build_server(true)
        .build_client(true)
        .compile_protos(
            &["../proto/semantic_cache.proto"],
            &["../proto"],
        )?;

    // Re-run the build script if the proto changes.
    println!("cargo:rerun-if-changed=../proto/semantic_cache.proto");

    Ok(())
}
