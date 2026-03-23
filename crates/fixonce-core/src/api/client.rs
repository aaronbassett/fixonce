/// Authenticated HTTP client for the `FixOnce` backend.
use reqwest::{header, Client, RequestBuilder};

use super::ApiError;

/// A configured HTTP client that carries the Supabase base URL and an
/// optional bearer token.
#[derive(Clone, Debug)]
pub struct ApiClient {
    pub(crate) base_url: String,
    pub(crate) token: Option<String>,
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

        Ok(Self {
            base_url: base_url.into(),
            token: None,
            http,
        })
    }

    /// Attach a JWT bearer token to every subsequent request.
    #[must_use]
    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    /// Return a [`RequestBuilder`] for a `GET` to `path` (relative to
    /// `base_url`), with the auth header pre-populated when a token is set.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError::Unauthenticated`] when no token is set.
    pub fn get_authenticated(&self, path: &str) -> Result<RequestBuilder, ApiError> {
        let token = self.token.as_deref().ok_or(ApiError::Unauthenticated)?;
        let url = format!("{}{}", self.base_url, path);
        Ok(self
            .http
            .get(url)
            .header(header::AUTHORIZATION, format!("Bearer {token}")))
    }

    /// Return a [`RequestBuilder`] for a `POST` to `path` (relative to
    /// `base_url`), with the auth header pre-populated when a token is set.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError::Unauthenticated`] when no token is set.
    pub fn post_authenticated(&self, path: &str) -> Result<RequestBuilder, ApiError> {
        let token = self.token.as_deref().ok_or(ApiError::Unauthenticated)?;
        let url = format!("{}{}", self.base_url, path);
        Ok(self
            .http
            .post(url)
            .header(header::AUTHORIZATION, format!("Bearer {token}")))
    }
}
