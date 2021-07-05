fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("../../massbit-core/proto/chaindata.proto")?;
    Ok(())
}