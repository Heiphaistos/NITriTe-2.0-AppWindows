use serde::{Deserialize, Serialize};
use crate::error::NiTriTeError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaChatMessage {
    pub role: String,    // "system" | "user" | "assistant"
    pub content: String,
}

pub async fn chat(
    url: &str,
    model: &str,
    messages: Vec<OllamaChatMessage>,
    temperature: f64,
) -> Result<String, NiTriTeError> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "model": model,
        "messages": messages,
        "stream": false,
        "options": { "temperature": temperature },
    });
    let resp = client
        .post(format!("{}/api/chat", url))
        .json(&body)
        .timeout(std::time::Duration::from_secs(180))
        .send()
        .await
        .map_err(|e| NiTriTeError::OllamaUnavailable(e.to_string()))?;
    // Le status doit être vérifié AVANT de parser le corps : Ollama renvoie un
    // JSON valide ({"error": "..."}) même sur 404/500 (ex: modèle inconnu),
    // donc resp.json() réussirait quand même — sans ce check, parse_chat_response
    // ne trouve ni "message.content" ni "response" dans un corps d'erreur et
    // retombe sur unwrap_or("") des deux côtés, renvoyant Ok("") : un faux succès
    // qui affichait une bulle de chat vide au lieu du vrai message d'erreur
    // Ollama (ex: "model 'xyz' not found"), contournant complètement le système
    // de messages d'erreur déjà en place côté frontend (AiAgentsPage.vue).
    let status = resp.status();
    let result: serde_json::Value = resp.json().await?;
    parse_chat_response(status, result)
}

fn parse_chat_response(status: reqwest::StatusCode, result: serde_json::Value) -> Result<String, NiTriTeError> {
    if !status.is_success() {
        let detail = result["error"].as_str().unwrap_or("réponse sans détail").to_string();
        return Err(NiTriTeError::OllamaUnavailable(format!("Ollama a renvoyé une erreur ({status}) : {detail}")));
    }
    // /api/chat returns {"message": {"role": "assistant", "content": "..."}}
    let content = result["message"]["content"].as_str().unwrap_or("").to_string();
    if content.is_empty() {
        // fallback au cas où la réponse a un format différent
        Ok(result["response"].as_str().unwrap_or("").to_string())
    } else {
        Ok(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_chat_response_extracts_message_content_on_success() {
        let body = serde_json::json!({"message": {"role": "assistant", "content": "Bonjour"}});
        let r = parse_chat_response(reqwest::StatusCode::OK, body).unwrap();
        assert_eq!(r, "Bonjour");
    }

    #[test]
    fn parse_chat_response_falls_back_to_response_field() {
        let body = serde_json::json!({"response": "Bonjour via generate"});
        let r = parse_chat_response(reqwest::StatusCode::OK, body).unwrap();
        assert_eq!(r, "Bonjour via generate");
    }

    #[test]
    fn parse_chat_response_errors_on_non_success_status_instead_of_returning_empty_ok() {
        // Format d'erreur réel documenté par l'API Ollama (404 modèle inconnu).
        // Reproduit le bug : sans le check de status, ce corps ne contient ni
        // message.content ni response, donc l'ancien code renvoyait Ok("").
        let body = serde_json::json!({"error": "model 'xyz' not found, try pulling it first"});
        let r = parse_chat_response(reqwest::StatusCode::NOT_FOUND, body);
        assert!(r.is_err(), "un status non-succès doit produire une Err, jamais un Ok(\"\") silencieux");
        assert!(format!("{}", r.unwrap_err()).contains("not found"));
    }

    #[test]
    fn parse_chat_response_errors_on_server_error_status_even_with_valid_json() {
        let body = serde_json::json!({"error": "internal server error"});
        let r = parse_chat_response(reqwest::StatusCode::INTERNAL_SERVER_ERROR, body);
        assert!(r.is_err());
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct OllamaModel {
    pub name: String,
    pub size_gb: f64,
    pub modified_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    pub url: String,
    pub model: String,
    pub temperature: f64,
    pub max_tokens: u32,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            url: "http://localhost:11434".into(),
            model: "llama3:8b".into(),
            temperature: 0.7,
            max_tokens: 2048,
        }
    }
}

pub async fn check_ollama(url: &str) -> bool {
    // Timeout court : reqwest::get() n'a AUCUN timeout par défaut, donc une URL
    // qui accepte la connexion sans répondre bloquerait ce health-check à vie.
    let Ok(client) = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
    else {
        return false;
    };
    client.get(format!("{}/api/tags", url)).send().await.is_ok()
}

pub async fn list_models(url: &str) -> Result<Vec<OllamaModel>, NiTriTeError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;
    let resp = client.get(format!("{}/api/tags", url)).send().await?;
    let body: serde_json::Value = resp.json().await?;

    let models = body["models"].as_array()
        .map(|arr| arr.iter().map(|m| OllamaModel {
            name: m["name"].as_str().unwrap_or("").to_string(),
            size_gb: m["size"].as_u64().unwrap_or(0) as f64 / 1_073_741_824.0,
            modified_at: m["modified_at"].as_str().unwrap_or("").to_string(),
        }).collect())
        .unwrap_or_default();

    Ok(models)
}

pub async fn query(
    url: &str,
    model: &str,
    prompt: &str,
    system_prompt: Option<&str>,
    temperature: f64,
) -> Result<String, NiTriTeError> {
    let client = reqwest::Client::new();

    let mut body = serde_json::json!({
        "model": model,
        "prompt": prompt,
        "stream": false,
        "options": { "temperature": temperature },
    });

    if let Some(sys) = system_prompt {
        body["system"] = serde_json::Value::String(sys.to_string());
    }

    let resp = client.post(format!("{}/api/generate", url))
        .json(&body)
        .timeout(std::time::Duration::from_secs(120))
        .send()
        .await
        .map_err(|e| NiTriTeError::OllamaUnavailable(e.to_string()))?;

    let result: serde_json::Value = resp.json().await?;
    Ok(result["response"].as_str().unwrap_or("").to_string())
}
