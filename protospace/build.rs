use protobuf_codegen_pure;
use std::fs;

fn main() {
    protobuf_codegen_pure::Codegen::new()
        .out_dir("src/protos/")
        .inputs(&["src/protos/client_api.proto"])
        .include("src/protos/")
        .run()
        .expect("Running protoc failed.");

    fs::rename(
        "src/protos/client_api.rs",
        "../spacetime_client_sdk/src/client_api.rs",
    )
    .unwrap();
}
