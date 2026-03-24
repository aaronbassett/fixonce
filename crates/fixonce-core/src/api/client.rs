/// Authenticated HTTP client for the `FixOnce` backend.
use reqwest::{header, Client, RequestBuilder};
use tracing::instrument;

use super::ApiError;

/// A configured HTTP client that carries the Supabase base URL and an
/// optional bearer token.
#[derive(Clone, Debug)]
pub struct ApiClient {
    pub(crate) base_url: String,
    pub(crate) token: Option<String>,
    pub(crate) anon_key: Option<String>,
    pub(crate) http: Client,
}

impl ApiClient {
    /// Create an unauthenticated client pointing at `base_url`.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError::Http`] if the underlying [`reqwest::Client`] cannot
    /// be built (e.g. no TLS support on this platform).
    pub fn new(base_url: impl Into<String>) -> Result<Self, ApiError> {
        let http = Client::builder().build().map_err(ApiError::Http)?;

        let anon_key = std::env::var("FIXONCE_ANON_KEY").ok();

        Ok(Self {
            base_url: base_url.into(),
            token: None,
            anon_key,
            http,
        })
    }

    /// Attach a JWT bearer token to every subsequent request.
    #[must_use]
    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    /// Attach the Supabase anon key (sent as `apikey` header on every request).
    #[must_use]
    pub fn with_anon_key(mut self, key: impl Into<String>) -> Self {
        self.anon_key = Some(key.into());
        self
    }

    /// Return a [`RequestBuilder`] for a `GET` to `path` (relative to
    /// `base_url`), with the auth header pre-populated when a token is set.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError::Unauthenticated`] when no token is set.
    #[instrument(skip(self), fields(http.method = "GET"))]
    pub fn get_authenticated(&self, path: &str) -> Result<RequestBuilder, ApiError> {
        let token = self.token.as_deref().ok_or(ApiError::Unauthenticated)?;
        let url = format!("{}{}", self.base_url, path);
        let mut req = self
            .http
            .get(url)
            .header(header::AUTHORIZATION, format!("Bearer {token}"));
        if let Some(ref key) = self.anon_key {
            req = req.header("apikey", key);
        }
        Ok(req)
    }

    /// Return a [`RequestBuilder`] for a `POST` to `path` (relative to
    /// `base_url`), with the auth header pre-populated when a token is set.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError::Unauthenticated`] when no token is set.
    #[instrument(skip(self), fields(http.method = "POST"))]
    pub fn post_authenticated(&self, path: &str) -> Result<RequestBuilder, ApiError> {
        let token = self.token.as_deref().ok_or(ApiError::Unauthenticated)?;
        let url = format!("{}{}", self.base_url, path);
        let mut req = self
            .http
            .post(url)
            .header(header::AUTHORIZATION, format!("Bearer {token}"));
        if let Some(ref key) = self.anon_key {
            req = req.header("apikey", key);
        }
        Ok(req)
    }
}
