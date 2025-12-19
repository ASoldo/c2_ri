fn main() {
    let protos = ["proto/mission.proto", "proto/task.proto"];
    let includes = ["proto"];

    let protoc = protoc_bin_vendored::protoc_bin_path().expect("protoc not available");
    unsafe {
        std::env::set_var("PROTOC", protoc);
    }

    prost_build::Config::new()
        .compile_protos(&protos, &includes)
        .expect("failed to compile proto definitions");

    for proto in &protos {
        println!("cargo:rerun-if-changed={}", proto);
    }
}
