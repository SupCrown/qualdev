
use tonic::Request;

// On inclut le code généré à partir du package "word" défini dans le proto
pub mod word {
    tonic::include_proto!("word");
}

use word::word_service_client::WordServiceClient;
use word::GetRandomWordRequest;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Connexion au serveur (assurez-vous que l'adresse et le port correspondent à votre serveur)
    // Par défaut, gRPC utilise souvent le port 50051 en local.
    let mut client = WordServiceClient::connect("http://0.0.0.0:50051").await?;

    // 2. Préparation de la requête
    // Le proto définit que la requête prend un paramètre 'length'
    let request = Request::new(GetRandomWordRequest {
        min_length: 0,
    });

    println!("Envoi de la requête pour un mot de 5 lettres...");

    // 3. Appel de la méthode RPC 'GetRandomWord'
    let response = client.get_random_word(request).await?;
    
    // 4. Affichage de la réponse
    println!("RÉPONSE REÇUE : {:?}", response.get_ref().word);

    Ok(())
}