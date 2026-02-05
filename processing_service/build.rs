fn main() -> Result<(), Box<dyn std::error::Error>> {
    // La fonction compile_protos se trouve maintenant dans tonic_prost_build
    tonic_prost_build::compile_protos("proto/word.proto")?;
    Ok(())
}