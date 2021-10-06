fn main() {
    println!("cargo:rerun-if-changed=proto");
    tonic_build::configure()
        .out_dir("src/firehose")
        .format(true)
        .compile(&["proto/stream.proto"], &["proto"])
        .expect("Failed to compile Firehose proto(s)");
}
