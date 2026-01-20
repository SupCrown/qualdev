use tonic::transport::Channel;
use anyhow::Result;

// On importe le module généré par le RPC (simulé ici par la macro include_proto)
// Cela correspond à la définition fournie par l'équipe Data Publish
pub mod data_publish_proto {
    tonic::include_proto!("word"); // Nom du package dans le fichier .proto
}

use data_publish_proto::dictionary_service_client::DictionaryServiceClient;
use data_publish_proto::EmptyRequest; // Une requête vide

pub struct DicoRpcClient {
    // Le client gRPC généré par tonic
    client: DictionaryServiceClient<Channel>,
}

impl DicoRpcClient {
    // Connexion au serveur RPC "Data Publish"
    pub async fn connect(dst: String) -> Result<Self> {
        let client = DictionaryServiceClient::connect(dst).await?;
        Ok(Self { client })
    }

    // Appel de la fonction précise
    pub async fn get_random_french_word(&mut self) -> Result<String> {
        // 1. On prépare la requête (vide ici car on veut juste un mot au hasard)
        let request = tonic::Request::new(EmptyRequest {});

        // 2. On appelle la fonction RPC définie par Data Publish
        let response = self.client.get_random_french_word(request).await?;

        // 3. On extrait le mot de la réponse
        // On suppose que la réponse a un champ nommé "word"
        let word = response.into_inner().word;

        Ok(word)
    }
}