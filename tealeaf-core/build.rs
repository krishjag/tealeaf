fn main() {
    let proto_path = "benches_proto/messages.proto";
    if std::path::Path::new(proto_path).exists() {
        prost_build::Config::new()
            .compile_protos(&[proto_path], &["benches_proto/"])
            .expect("Failed to compile protobuf");
    }
}
