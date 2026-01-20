fn main() -> Result<(), Box<dyn std::error::Error>> {
    // On pointe vers le fichier proto du dossier voisin "word_service"
    let proto_file = "../word_service/proto/word.proto";

    // On compile le proto pour pouvoir l'utiliser dans le code Rust
    tonic_build::compile_protos(proto_file)?;

    println!("cargo:rerun-if-changed={}", proto_file);

    Ok(())
}