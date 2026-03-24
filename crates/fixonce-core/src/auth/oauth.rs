/// GitHub OAuth browser flow (backend-brokered).
///
/// The CLI never calls Supabase auth endpoints directly.  Instead it asks the
/// `FixOnce` backend for the OAuth URL (`auth-login-start`) and sends the
/// received authorisation code back to the backend for exchange
/// (`auth-login-exchange`).  This keeps all Supabase auth details — including
/// the anon key — on the server side.
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use super::AuthError;

/// Perform the GitHub OAuth flow via the `FixOnce` backend and return a JWT.
///
/// 1. Binds a random-port local HTTP server for the OAuth callback.
/// 2. Asks the backend for the OAuth authorization URL.
/// 3. Opens the user's browser to that URL.
/// 4. Waits for the callback with the authorization code.
/// 5. Sends the code to the backend for exchange.
/// 6. Returns the JWT access token.
///
/// # Errors
///
/// Returns [`AuthError::OAuthFailed`] when the browser cannot be opened, the
/// local callback server fails to bind, or the OAuth exchange returns an error.
/// Returns [`AuthError::HttpError`] when an HTTP request to the backend fails.
pub async fn login_with_github(api_url: &str) -> Result<String, AuthError> {
    // Bind to a random high port for the local callback server.
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| AuthError::OAuthFailed(format!("failed to bind callback server: {e}")))?;

    let port = listener
        .local_addr()
        .map_err(|e| AuthError::OAuthFailed(format!("cannot read local port: {e}")))?
        .port();

    let redirect_uri = format!("http://127.0.0.1:{port}/callback");

    // Step 1: Ask the backend for the OAuth authorization URL.
    let auth_url = fetch_auth_url(api_url, &redirect_uri).await?;

    // Step 2: Launch the browser.
    open::that(&auth_url)
        .map_err(|e| AuthError::OAuthFailed(format!("cannot open browser: {e}")))?;

    println!("Opening browser for GitHub OAuth…");
    println!("If the browser did not open, visit:\n  {auth_url}");

    // Step 3: Wait for the callback request from the browser.
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

    // Step 4: Exchange the authorisation code for a JWT via the backend.
    let jwt = exchange_code_for_jwt(api_url, &code, &redirect_uri).await?;
    Ok(jwt)
}

/// Ask the backend for the OAuth authorization URL.
async fn fetch_auth_url(api_url: &str, redirect_uri: &str) -> Result<String, AuthError> {
    let client = reqwest::Client::new();
    let url = format!("{api_url}/functions/v1/auth-login-start");

    let body = serde_json::json!({ "redirect_uri": redirect_uri });

    let response = client
        .post(&url)
        .json(&body)
        .send()
        .await?
        .error_for_status()
        .map_err(|e| AuthError::OAuthFailed(format!("auth-login-start failed: {e}")))?;

    let payload: serde_json::Value = response.json().await?;

    payload["auth_url"]
        .as_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            AuthError::OAuthFailed(format!(
                "auth-login-start response missing auth_url: {payload}"
            ))
        })
}

/// Exchange an OAuth authorisation `code` for a JWT via the backend.
async fn exchange_code_for_jwt(
    api_url: &str,
    code: &str,
    redirect_uri: &str,
) -> Result<String, AuthError> {
    let client = reqwest::Client::new();
    let url = format!("{api_url}/functions/v1/auth-login-exchange");

    let body = serde_json::json!({
        "code": code,
        "redirect_uri": redirect_uri,
    });

    let response = client
        .post(&url)
        .json(&body)
        .send()
        .await?
        .error_for_status()
        .map_err(|e| AuthError::OAuthFailed(format!("auth-login-exchange failed: {e}")))?;

    let payload: serde_json::Value = response.json().await?;

    payload["access_token"]
        .as_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            AuthError::OAuthFailed(format!(
                "auth-login-exchange response missing access_token: {payload}"
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
