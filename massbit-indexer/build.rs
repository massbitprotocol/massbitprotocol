fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("proto/streamout.proto")?;
    // tonic_build::configure()
    //     .build_server(false)
    //     .out_dir("src")  // you can change the generated code's location
    //     .compile(
    //         &["proto/helloworld.proto"],
    //         &["proto"], // specify the root location to search proto dependencies
    //     ).unwrap();
    Ok(())
}