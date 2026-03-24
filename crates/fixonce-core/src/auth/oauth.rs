/// GitHub OAuth browser flow.
///
/// Opens the user's default browser to the GitHub OAuth consent page, spins up
/// a short-lived local HTTP server to receive the callback, then exchanges the
/// authorisation code for a JWT via the Supabase auth edge function.
use std::collections::HashMap;

use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use super::AuthError;

/// Generate a PKCE code verifier (43–128 chars, URL-safe).
fn generate_code_verifier() -> String {
    use rand::Rng;
    let mut rng = rand::rng();
    let bytes: Vec<u8> = (0..32).map(|_| rng.random::<u8>()).collect();
    base64_url_encode(&bytes)
}

/// Compute the S256 code challenge from a code verifier.
fn compute_code_challenge(verifier: &str) -> String {
    let digest = Sha256::digest(verifier.as_bytes());
    base64_url_encode(&digest)
}

/// Base64-URL encode without padding (per RFC 7636).
fn base64_url_encode(input: &[u8]) -> String {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine;
    URL_SAFE_NO_PAD.encode(input)
}

/// Perform the GitHub OAuth flow and return a JWT on success.
///
/// # Errors
///
/// Returns [`AuthError::OAuthFailed`] when the browser cannot be opened, the
/// local callback server fails to bind, or the OAuth exchange returns an error.
/// Returns [`AuthError::HttpError`] when the token-exchange HTTP request fails.
pub async fn login_with_github(supabase_url: &str, anon_key: &str) -> Result<String, AuthError> {
    // Bind to a random high port for the local callback server.
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| AuthError::OAuthFailed(format!("failed to bind callback server: {e}")))?;

    let port = listener
        .local_addr()
        .map_err(|e| AuthError::OAuthFailed(format!("cannot read local port: {e}")))?
        .port();

    let redirect_uri = format!("http://127.0.0.1:{port}/callback");

    // PKCE: generate verifier and challenge.
    let code_verifier = generate_code_verifier();
    let code_challenge = compute_code_challenge(&code_verifier);

    // Build the GitHub OAuth URL with PKCE parameters.
    let auth_url = format!(
        "{supabase_url}/auth/v1/authorize?provider=github&redirect_to={redirect_uri}&flow_type=pkce&code_challenge={code_challenge}&code_challenge_method=S256"
    );

    // Launch the browser.
    open::that(&auth_url)
        .map_err(|e| AuthError::OAuthFailed(format!("cannot open browser: {e}")))?;

    println!("Opening browser for GitHub OAuth…");
    println!("If the browser did not open, visit:\n  {auth_url}");

    // Wait for the callback request from the browser.
    let (mut stream, _) = listener
        .accept()
        .await
        .map_err(|e| AuthError::OAuthFailed(format!("callback accept failed: {e}")))?;

    let mut buf = vec![0u8; 4096];
    let n = stream
        .read(&mut buf)
        .await
        .map_err(|e| AuthError::OAuthFailed(format!("callback read failed: {e}")))?;

    let request = String::from_utf8_lossy(&buf[..n]);
    let code = extract_query_param(&request, "code").ok_or_else(|| {
        let err = extract_query_param(&request, "error")
            .unwrap_or_else(|| "no code in callback".to_owned());
        AuthError::OAuthFailed(err)
    })?;

    // Acknowledge the browser so it doesn't hang.
    let html_response = concat!(
        "HTTP/1.1 200 OK\r\n",
        "Content-Type: text/html; charset=utf-8\r\n",
        "\r\n",
        "<html><body><h2>Authentication successful — you can close this tab.</h2></body></html>",
    );
    let _ = stream.write_all(html_response.as_bytes()).await;

    // Exchange the authorisation code for a JWT.
    let jwt = exchange_code_for_jwt(supabase_url, anon_key, &code, &redirect_uri, &code_verifier).await?;
    Ok(jwt)
}

/// Exchange an OAuth authorisation `code` for a Supabase JWT.
async fn exchange_code_for_jwt(
    supabase_url: &str,
    anon_key: &str,
    code: &str,
    redirect_uri: &str,
    code_verifier: &str,
) -> Result<String, AuthError> {
    let client = reqwest::Client::new();
    let url = format!("{supabase_url}/auth/v1/token?grant_type=pkce");

    let mut body = HashMap::new();
    body.insert("auth_code", code);
    body.insert("code_verifier", code_verifier);
    body.insert("redirect_uri", redirect_uri);

    let response = client
        .post(&url)
        .header("apikey", anon_key)
        .json(&body)
        .send()
        .await?
        .error_for_status()
        .map_err(|e| AuthError::OAuthFailed(format!("token exchange HTTP error: {e}")))?;

    let payload: serde_json::Value = response.json().await?;

    payload["access_token"]
        .as_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            AuthError::OAuthFailed(format!(
                "token exchange response missing access_token: {payload}"
            ))
        })
}

/// Extract a single named query parameter value from a raw HTTP request line.
///
/// The first line of an HTTP request looks like `GET /callback?code=abc&state=xyz HTTP/1.1`.
fn extract_query_param(raw_request: &str, name: &str) -> Option<String> {
    let first_line = raw_request.lines().next()?;
    // First line: "GET /path?key=value HTTP/1.1"
    let path = first_line.split_whitespace().nth(1)?;
    let query = path.split_once('?')?.1;

    for pair in query.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            if k == name {
                return Some(v.to_owned());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_code_from_request() {
        let req = "GET /callback?code=abc123&state=xyz HTTP/1.1\r\nHost: localhost\r\n\r\n";
        assert_eq!(extract_query_param(req, "code"), Some("abc123".to_owned()));
    }

    #[test]
    fn extract_missing_param_returns_none() {
        let req = "GET /callback?state=xyz HTTP/1.1\r\n\r\n";
        assert_eq!(extract_query_param(req, "code"), None);
    }
}
