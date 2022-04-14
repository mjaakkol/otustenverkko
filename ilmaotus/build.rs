//use protoc_rust::Customize;

fn main() {
    protoc_rust::Codegen::new()
        .out_dir("src")
        .inputs(&["../protos/messages.proto"])
        .include("../protos")
        .run()
        .expect("protoc")
}
