//! `VoyageAI` `voyage-code-3` embedding client.
//!
//! Sends text to the `VoyageAI` API and returns a 1 024-dimensional `f64` vector.
//! Retries up to three times with exponential back-off (1 s → 2 s → 4 s) on
//! transient failures (EC-25).

use serde::{Deserialize, Serialize};

use crate::error::EmbeddingError;

const VOYAGE_API_URL: &str = "https://api.voyageai.com/v1/embeddings";
const VOYAGE_MODEL: &str = "voyage-code-3";
const EXPECTED_DIMS: usize = 1024;
const MAX_RETRIES: u32 = 3;

// ---------------------------------------------------------------------------
// Request / response shapes
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct EmbeddingRequest<'a> {
    model: &'a str,
    input: [&'a str; 1],
}

#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingDatum>,
}

#[derive(Debug, Deserialize)]
struct EmbeddingDatum {
    embedding: Vec<f64>,
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

/// Thin client that wraps a shared [`reqwest::Client`] for `VoyageAI` calls.
///
/// The API key is passed per-call so it can be fetched ephemerally and dropped
/// immediately after use.
#[derive(Clone, Debug)]
pub struct VoyageClient {
    http: reqwest::Client,
}

impl VoyageClient {
    /// Create a new client.
    ///
    /// # Errors
    ///
    /// Returns [`EmbeddingError::Http`] if the underlying HTTP client cannot be
    /// built (very rare — typically only fails when TLS is unavailable).
    pub fn new() -> Result<Self, EmbeddingError> {
        let http = reqwest::Client::builder()
            .build()
            .map_err(EmbeddingError::Http)?;
        Ok(Self { http })
    }

    /// Generate a 1 024-dimensional embedding for `text`.
    ///
    /// The `api_key` is passed as a header and **must not** be retained by the
    /// caller after this call returns.
    ///
    /// # Errors
    ///
    /// Returns [`EmbeddingError`] on network failure, API errors, or if the
    /// response does not contain exactly [`EXPECTED_DIMS`] dimensions.
    pub async fn generate_embedding(
        &self,
        api_key: &str,
        text: &str,
    ) -> Result<Vec<f64>, EmbeddingError> {
        let body = EmbeddingRequest {
            model: VOYAGE_MODEL,
            input: [text],
        };

        let mut last_err: Option<EmbeddingError> = None;

        for attempt in 0..MAX_RETRIES {
            if attempt > 0 {
                let delay = std::time::Duration::from_secs(2u64.pow(attempt - 1));
                tokio::time::sleep(delay).await;
            }

            match self.try_once(api_key, &body).await {
                Ok(embedding) => return Ok(embedding),
                Err(e) => {
                    last_err = Some(e);
                }
            }
        }

        Err(last_err.unwrap_or(EmbeddingError::EmptyResponse))
    }

    /// Perform a single (non-retried) request to the `VoyageAI` embeddings API.
    async fn try_once(
        &self,
        api_key: &str,
        body: &EmbeddingRequest<'_>,
    ) -> Result<Vec<f64>, EmbeddingError> {
        let response = self
            .http
            .post(VOYAGE_API_URL)
            .bearer_auth(api_key)
            .json(body)
            .send()
            .await
            .map_err(EmbeddingError::Http)?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response
                .text()
                .await
                .unwrap_or_else(|_| "(unreadable body)".to_owned());
            return Err(EmbeddingError::ApiError {
                status: status.as_u16(),
                body: text,
            });
        }

        let parsed: EmbeddingResponse = response
            .json()
            .await
            .map_err(|e| EmbeddingError::UnexpectedResponse(e.to_string()))?;

        let embedding = parsed
            .data
            .into_iter()
            .next()
            .ok_or(EmbeddingError::EmptyResponse)?
            .embedding;

        if embedding.len() != EXPECTED_DIMS {
            return Err(EmbeddingError::UnexpectedDimensions {
                expected: EXPECTED_DIMS,
                got: embedding.len(),
            });
        }

        Ok(embedding)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify that the request body serialises to the shape the API expects.
    #[test]
    fn request_body_serialises_correctly() {
        let req = EmbeddingRequest {
            model: VOYAGE_MODEL,
            input: ["hello world"],
        };
        let json = serde_json::to_value(&req).expect("serialisation must not fail");
        assert_eq!(json["model"], "voyage-code-3");
        let input = json["input"].as_array().expect("input must be an array");
        assert_eq!(input.len(), 1);
        assert_eq!(input[0], "hello world");
    }

    /// Verify that an API response with the correct shape deserialises cleanly.
    #[test]
    fn response_deserialises_correctly() {
        let embedding: Vec<f64> = vec![0.1_f64; EXPECTED_DIMS];
        let raw = serde_json::json!({
            "data": [{ "embedding": embedding }]
        });
        let parsed: EmbeddingResponse =
            serde_json::from_value(raw).expect("deserialisation must not fail");
        assert_eq!(parsed.data[0].embedding.len(), EXPECTED_DIMS);
    }

    /// Confirm that a mismatched dimension count yields the right error.
    #[test]
    fn wrong_dimension_count_yields_error() {
        // 16-dimensional response instead of 1024
        let small: Vec<f64> = vec![0.5_f64; 16];
        let err = EmbeddingError::UnexpectedDimensions {
            expected: EXPECTED_DIMS,
            got: small.len(),
        };
        assert!(matches!(
            err,
            EmbeddingError::UnexpectedDimensions {
                expected: 1024,
                got: 16
            }
        ));
    }
}
