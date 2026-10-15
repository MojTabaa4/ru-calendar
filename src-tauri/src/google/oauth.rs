//! Google sign-in for a desktop app: the authorization-code flow with PKCE, the
//! browser returning to a one-shot web server on 127.0.0.1 (Google's "loopback" flow).

use std::io::{ErrorKind, Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use reqwest::Url;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use super::GoogleError;
use super::config::ClientConfig;

pub const AUTH_ENDPOINT: &str = "https://accounts.google.com/o/oauth2/v2/auth";
pub const TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";
pub const REVOKE_ENDPOINT: &str = "https://oauth2.googleapis.com/revoke";
/// Read-only calendars, plus the account's email to tell accounts apart.
pub const SCOPES: &str = "openid email https://www.googleapis.com/auth/calendar.readonly";

fn random_token() -> String {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).expect("the OS random number generator works");
    URL_SAFE_NO_PAD.encode(bytes)
}

/// A PKCE verifier and its S256 challenge.
pub fn pkce() -> (String, String) {
    let verifier = random_token();
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    (verifier, challenge)
}

pub fn new_state() -> String {
    random_token()
}

pub fn auth_url(client: &ClientConfig, redirect_uri: &str, challenge: &str, state: &str) -> Url {
    Url::parse_with_params(
        AUTH_ENDPOINT,
        &[
            ("client_id", client.client_id.as_str()),
            ("redirect_uri", redirect_uri),
            ("response_type", "code"),
            ("scope", SCOPES),
            ("code_challenge", challenge),
            ("code_challenge_method", "S256"),
            ("state", state),
            // A refresh token, so Ruz stays signed in.
            ("access_type", "offline"),
            ("prompt", "consent select_account"),
        ],
    )
    .expect("the endpoint is a valid URL")
}

/// What the browser brought back to the loopback server.
#[derive(Debug, PartialEq, Eq)]
pub enum Callback {
    Code(String),
    Denied,
    /// Not the redirect (e.g. a favicon request); keep waiting.
    Other,
}

/// Reads the request line of an HTTP request such as `GET /?state=..&code=.. HTTP/1.1`.
pub fn parse_callback(request: &str, expected_state: &str) -> Result<Callback, GoogleError> {
    let Some(target) = request.lines().next().and_then(|line| line.split_whitespace().nth(1)) else {
        return Ok(Callback::Other);
    };
    let url = Url::parse(&format!("http://127.0.0.1{target}"))
        .map_err(|_| GoogleError::Sign("bad callback".into()))?;
    let get = |key: &str| url.query_pairs().find(|(k, _)| k == key).map(|(_, v)| v.into_owned());
    if get("error").is_some() {
        return Ok(Callback::Denied);
    }
    let Some(code) = get("code") else { return Ok(Callback::Other) };
    if get("state").as_deref() != Some(expected_state) {
        // Not our request: possibly forged, so never use its code.
        return Err(GoogleError::Sign("the sign-in response didn't match".into()));
    }
    Ok(Callback::Code(code))
}

const DONE_PAGE: &str = "<!doctype html><html dir=\"rtl\" lang=\"fa\"><meta charset=\"utf-8\">\
<title>روز</title><body style=\"font-family:system-ui;text-align:center;padding:4em\">\
<h2>روز به حساب گوگل شما وصل شد.</h2><p>می‌توانید این زبانه را ببندید.</p></body></html>";

const DENIED_PAGE: &str = "<!doctype html><html dir=\"rtl\" lang=\"fa\"><meta charset=\"utf-8\">\
<title>روز</title><body style=\"font-family:system-ui;text-align:center;padding:4em\">\
<h2>اتصال به گوگل انجام نشد.</h2><p>می‌توانید این زبانه را ببندید.</p></body></html>";

fn respond(stream: &mut std::net::TcpStream, body: &str) {
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
}

/// Waits for the browser to come back with the authorization code, until `timeout` or
/// until `cancelled` is set. Blocks.
pub fn wait_for_code(
    listener: TcpListener,
    state: &str,
    timeout: Duration,
    cancelled: &AtomicBool,
) -> Result<String, GoogleError> {
    listener.set_nonblocking(true).map_err(GoogleError::io)?;
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if cancelled.load(Ordering::SeqCst) {
            return Err(GoogleError::Cancelled);
        }
        let mut stream = match listener.accept() {
            Ok((stream, _)) => stream,
            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }
            Err(e) => return Err(GoogleError::io(e)),
        };
        let _ = stream.set_nonblocking(false);
        let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
        let mut buffer = [0u8; 8192];
        let n = stream.read(&mut buffer).unwrap_or(0);
        let request = String::from_utf8_lossy(&buffer[..n]);
        match parse_callback(&request, state)? {
            Callback::Code(code) => {
                respond(&mut stream, DONE_PAGE);
                return Ok(code);
            }
            Callback::Denied => {
                respond(&mut stream, DENIED_PAGE);
                return Err(GoogleError::Denied);
            }
            Callback::Other => respond(&mut stream, ""),
        }
    }
    Err(GoogleError::Sign("the sign-in took too long".into()))
}

#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub expires_in: u64,
    pub refresh_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TokenError {
    error: String,
}

async fn token_request(http: &reqwest::Client, form: &[(&str, &str)]) -> Result<TokenResponse, GoogleError> {
    let response = http.post(TOKEN_ENDPOINT).form(form).send().await.map_err(GoogleError::network)?;
    if response.status().is_success() {
        return response.json().await.map_err(GoogleError::network);
    }
    let error = response.json::<TokenError>().await.map(|e| e.error).unwrap_or_default();
    // A revoked or expired sign-in has to be redone.
    Err(if error == "invalid_grant" { GoogleError::SignedOut } else { GoogleError::Sign(error) })
}

pub async fn exchange_code(
    http: &reqwest::Client,
    client: &ClientConfig,
    code: &str,
    verifier: &str,
    redirect_uri: &str,
) -> Result<TokenResponse, GoogleError> {
    token_request(
        http,
        &[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("code_verifier", verifier),
            ("redirect_uri", redirect_uri),
            ("client_id", &client.client_id),
            ("client_secret", &client.client_secret),
        ],
    )
    .await
}

pub async fn refresh(
    http: &reqwest::Client,
    client: &ClientConfig,
    refresh_token: &str,
) -> Result<TokenResponse, GoogleError> {
    token_request(
        http,
        &[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", &client.client_id),
            ("client_secret", &client.client_secret),
        ],
    )
    .await
}

/// Tells Google to forget the sign-in. Best effort: the token is deleted locally anyway.
pub async fn revoke(http: &reqwest::Client, token: &str) {
    let _ = http.post(REVOKE_ENDPOINT).form(&[("token", token)]).send().await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_pair() {
        let (verifier, challenge) = pkce();
        assert_eq!(verifier.len(), 43);
        assert_eq!(challenge, URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes())));
        assert_ne!(pkce().0, verifier, "each sign-in gets a fresh verifier");
    }

    #[test]
    fn consent_url() {
        let client =
            ClientConfig { client_id: "id.apps.googleusercontent.com".into(), client_secret: "s".into() };
        let url = auth_url(&client, "http://127.0.0.1:5555", "chal", "st");
        let get = |k: &str| url.query_pairs().find(|(key, _)| key == k).map(|(_, v)| v.into_owned());
        assert_eq!(get("client_id").as_deref(), Some("id.apps.googleusercontent.com"));
        assert_eq!(get("redirect_uri").as_deref(), Some("http://127.0.0.1:5555"));
        assert_eq!(get("code_challenge_method").as_deref(), Some("S256"));
        assert_eq!(get("access_type").as_deref(), Some("offline"));
        assert!(get("scope").unwrap().contains("calendar.readonly"));
        assert!(get("client_secret").is_none(), "the secret never goes to the browser");
    }

    #[test]
    fn callbacks() {
        let ok = "GET /?state=abc&code=4%2F0Ab&scope=email HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n";
        assert_eq!(parse_callback(ok, "abc").unwrap(), Callback::Code("4/0Ab".into()));
        assert!(parse_callback(ok, "other").is_err(), "a mismatched state is rejected");
        let denied = "GET /?error=access_denied&state=abc HTTP/1.1\r\n\r\n";
        assert_eq!(parse_callback(denied, "abc").unwrap(), Callback::Denied);
        assert_eq!(parse_callback("GET /favicon.ico HTTP/1.1\r\n\r\n", "abc").unwrap(), Callback::Other);
    }

    #[test]
    fn loopback_server_returns_the_code() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let browser = std::thread::spawn(move || {
            let mut favicon = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
            favicon.write_all(b"GET /favicon.ico HTTP/1.1\r\n\r\n").unwrap();
            let mut page = String::new();
            let _ = favicon.read_to_string(&mut page);
            let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
            stream.write_all(b"GET /?state=s1&code=xyz HTTP/1.1\r\n\r\n").unwrap();
            let mut page = String::new();
            stream.read_to_string(&mut page).unwrap();
            page
        });
        let code = wait_for_code(listener, "s1", Duration::from_secs(5), &AtomicBool::new(false)).unwrap();
        assert_eq!(code, "xyz");
        assert!(browser.join().unwrap().contains("200 OK"));
    }

    #[test]
    fn waiting_can_be_cancelled() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let result = wait_for_code(listener, "s1", Duration::from_secs(5), &AtomicBool::new(true));
        assert_eq!(result, Err(GoogleError::Cancelled));
    }
}
