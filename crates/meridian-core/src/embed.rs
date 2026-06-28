use anyhow::Result;

/// Compute embeddings for a batch of texts. Each embedding is a float vector.
pub trait Embedder: Send + Sync {
    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>>;

    fn embed_one(&self, text: &str) -> Result<Vec<f32>> {
        let mut batch = self.embed_batch(&[text])?;
        batch
            .pop()
            .ok_or_else(|| anyhow::anyhow!("embedder returned empty batch"))
    }
}

/// Always returns zero vectors — useful for testing and as a null backend.
pub struct NullEmbedder {
    pub dims: usize,
}

impl Embedder for NullEmbedder {
    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        Ok(texts.iter().map(|_| vec![0.0f32; self.dims]).collect())
    }
}

/// Voyage AI embedding backend.
/// Requires VOYAGE_API_KEY env var or explicit api_key.
/// Models: voyage-3-lite (cheapest), voyage-3, voyage-2
#[cfg(feature = "voyage")]
pub struct VoyageEmbedder {
    api_key: String,
    model: String,
    client: reqwest::blocking::Client,
}

#[cfg(feature = "voyage")]
impl VoyageEmbedder {
    pub fn new(api_key: Option<String>, model: Option<String>) -> Result<Self> {
        let api_key = api_key
            .or_else(|| std::env::var("VOYAGE_API_KEY").ok())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Voyage AI API key required.\n\
                     Set VOYAGE_API_KEY env var or add api_key to [embedding] in meridian.toml."
                )
            })?;
        let model = model.unwrap_or_else(|| "voyage-3-lite".to_string());
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;
        Ok(Self {
            api_key,
            model,
            client,
        })
    }
}

#[cfg(feature = "voyage")]
impl Embedder for VoyageEmbedder {
    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        use serde::{Deserialize, Serialize};

        #[derive(Serialize)]
        struct Request<'a> {
            input: Vec<&'a str>,
            model: &'a str,
        }

        #[derive(Deserialize)]
        struct Response {
            data: Vec<EmbeddingData>,
        }

        #[derive(Deserialize)]
        struct EmbeddingData {
            embedding: Vec<f32>,
        }

        // Voyage allows up to 128 inputs per request
        let mut results = Vec::with_capacity(texts.len());
        for chunk in texts.chunks(128) {
            let req = Request {
                input: chunk.to_vec(),
                model: &self.model,
            };
            let resp: Response = self
                .client
                .post("https://api.voyageai.com/v1/embeddings")
                .header("Authorization", format!("Bearer {}", self.api_key))
                .json(&req)
                .send()
                .map_err(|e| anyhow::anyhow!("Voyage AI request failed: {e}"))?
                .error_for_status()
                .map_err(|e| anyhow::anyhow!("Voyage AI error: {e}"))?
                .json()
                .map_err(|e| anyhow::anyhow!("Voyage AI response parse error: {e}"))?;
            results.extend(resp.data.into_iter().map(|d| d.embedding));
        }
        Ok(results)
    }
}

pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_embedder_returns_correct_dims() {
        let e = NullEmbedder { dims: 4 };
        let vecs = e.embed_batch(&["hello", "world"]).unwrap();
        assert_eq!(vecs.len(), 2);
        assert_eq!(vecs[0].len(), 4);
    }

    #[test]
    fn cosine_identical() {
        let v = vec![1.0f32, 0.0, 1.0];
        assert!((cosine(&v, &v) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn cosine_orthogonal() {
        let a = vec![1.0f32, 0.0];
        let b = vec![0.0f32, 1.0];
        assert!(cosine(&a, &b).abs() < 1e-6);
    }

    #[test]
    fn embed_one_delegates_to_batch() {
        let e = NullEmbedder { dims: 8 };
        let v = e.embed_one("hello").unwrap();
        assert_eq!(v.len(), 8);
    }
}
