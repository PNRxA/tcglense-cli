//! Browser-based sign-in: the OAuth 2.0 native-app **loopback** flow (RFC 8252)
//! with PKCE, so `tcglense login` never takes a password in the terminal.
//!
//! The CLI binds a short-lived HTTP listener on `127.0.0.1:<random port>`, opens
//! the browser to the site's `/cli-login` page carrying that loopback
//! `redirect_uri`, an anti-CSRF `state`, and a PKCE `code_challenge`
//! (`SHA-256(verifier)`). The user authenticates + approves in the browser; the
//! page redirects back to the loopback URL with a one-time `code`, which the CLI
//! then exchanges (presenting the private `verifier`) for a real session. Because
//! the verifier never leaves the CLI, a code observed on the loopback URL is
//! useless to anyone else.

use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use rand::RngCore;
use sha2::{Digest, Sha256};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    time::timeout,
};

/// How long to wait for the browser round-trip before giving up. Matches the
/// server's authorization-code lifetime.
const CALLBACK_TIMEOUT: Duration = Duration::from_secs(300);

/// One in-progress browser login: the loopback listener plus the PKCE + CSRF
/// secrets that bind the eventual code exchange to this exact CLI invocation.
pub struct BrowserLogin {
    listener: TcpListener,
    port: u16,
    /// Anti-CSRF nonce echoed back on the loopback redirect.
    state: String,
    /// The PKCE verifier — kept private; presented only at the token exchange.
    pub verifier: String,
    /// `SHA-256(verifier)` in hex — handed to the browser/server up front.
    challenge: String,
}

impl BrowserLogin {
    /// Bind the loopback listener and mint fresh PKCE + CSRF secrets.
    pub async fn bind() -> Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", 0))
            .await
            .context("could not open a local port to receive the login redirect")?;
        let port = listener
            .local_addr()
            .context("could not read the local login port")?
            .port();
        let verifier = random_hex();
        let challenge = sha256_hex(&verifier);
        Ok(Self {
            listener,
            port,
            state: random_hex(),
            verifier,
            challenge,
        })
    }

    /// The loopback URL the browser is told to return to.
    fn redirect_uri(&self) -> String {
        format!("http://127.0.0.1:{}/callback", self.port)
    }

    /// The absolute sign-in URL to open in the browser, against `site_base` (the
    /// origin that serves the SPA — the same origin the CLI talks to).
    pub fn authorize_url(&self, site_base: &str, client_name: &str) -> Result<String> {
        let base = site_base.trim_end_matches('/');
        let mut url = reqwest::Url::parse(&format!("{base}/cli-login"))
            .with_context(|| format!("invalid base URL: {site_base}"))?;
        url.query_pairs_mut()
            .append_pair("redirect_uri", &self.redirect_uri())
            .append_pair("state", &self.state)
            .append_pair("code_challenge", &self.challenge)
            .append_pair("name", client_name);
        Ok(url.to_string())
    }

    /// Wait for the browser to hit the loopback `/callback`, validate `state`, and
    /// return the one-time authorization `code`. Times out after
    /// [`CALLBACK_TIMEOUT`]; a user cancellation comes back as an `error` param.
    pub async fn wait_for_code(&self) -> Result<String> {
        timeout(CALLBACK_TIMEOUT, self.accept_callback())
            .await
            .map_err(|_| {
                anyhow!("timed out waiting for the browser sign-in (no response in 5 minutes)")
            })?
    }

    /// Accept loopback connections until one carries the `/callback` redirect.
    async fn accept_callback(&self) -> Result<String> {
        loop {
            let (stream, _) = self
                .listener
                .accept()
                .await
                .context("failed to accept the login redirect connection")?;
            if let Some(result) = self.handle_connection(stream).await? {
                // A definite outcome (the code, or a hard error) — stop listening.
                return result;
            }
            // A stray request (e.g. a favicon probe) — keep waiting for /callback.
        }
    }

    /// Parse one HTTP request. Returns `Some(Ok(code))` / `Some(Err(..))` once the
    /// `/callback` is seen, or `None` for any other path (keep listening).
    async fn handle_connection(&self, mut stream: TcpStream) -> Result<Option<Result<String>>> {
        let Some(target) = read_request_target(&mut stream).await? else {
            return Ok(None);
        };

        // Resolve the path + query against a dummy base to reuse the URL parser.
        let url = reqwest::Url::parse(&format!("http://127.0.0.1{target}"))
            .context("malformed login redirect request")?;
        if url.path() != "/callback" {
            respond(&mut stream, 404, PAGE_ERROR).await;
            return Ok(None);
        }

        let mut code = None;
        let mut state = None;
        let mut error = None;
        for (k, v) in url.query_pairs() {
            match k.as_ref() {
                "code" => code = Some(v.into_owned()),
                "state" => state = Some(v.into_owned()),
                "error" => error = Some(v.into_owned()),
                _ => {}
            }
        }

        if let Some(err) = error {
            respond(&mut stream, 400, PAGE_DENIED).await;
            return Ok(Some(Err(anyhow!(
                "sign-in was declined in the browser ({err})"
            ))));
        }

        // Reject a mismatched/absent state — guards against a stray or forged
        // callback landing on the loopback port.
        if state.as_deref() != Some(self.state.as_str()) {
            respond(&mut stream, 400, PAGE_ERROR).await;
            return Ok(Some(Err(anyhow!(
                "login redirect state did not match; aborting for safety"
            ))));
        }

        match code {
            Some(code) if !code.is_empty() => {
                respond(&mut stream, 200, PAGE_OK).await;
                Ok(Some(Ok(code)))
            }
            _ => {
                respond(&mut stream, 400, PAGE_ERROR).await;
                Ok(Some(Err(anyhow!(
                    "login redirect carried no authorization code"
                ))))
            }
        }
    }
}

/// A best-effort label for the device, shown on the browser consent screen.
pub fn client_label() -> String {
    let host = std::env::var("HOSTNAME")
        .ok()
        .or_else(|| std::env::var("COMPUTERNAME").ok()) // Windows
        .map(|h| h.trim().to_string())
        .filter(|h| !h.is_empty())
        .unwrap_or_else(|| "this computer".to_string());
    format!("tcglense CLI on {host}")
}

/// Open `url` in the user's default browser (best effort).
pub fn open_browser(url: &str) -> Result<()> {
    webbrowser::open(url).map_err(|e| anyhow!("could not open a browser: {e}"))?;
    Ok(())
}

/// 32 CSPRNG bytes, hex-encoded (64 hex chars) — the same shape as the server's
/// opaque secrets.
fn random_hex() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

fn sha256_hex(input: &str) -> String {
    hex::encode(Sha256::digest(input.as_bytes()))
}

/// Read the HTTP request line and return the request target (the path+query),
/// e.g. `/callback?code=…`. Returns `None` if the request is empty/unparseable.
async fn read_request_target(stream: &mut TcpStream) -> Result<Option<String>> {
    // The request line is tiny and arrives first; a bounded read avoids a slow
    // client holding the accept loop.
    let mut buf = [0u8; 4096];
    let n = timeout(Duration::from_secs(10), stream.read(&mut buf))
        .await
        .map_err(|_| anyhow!("the browser connection stalled"))?
        .context("failed to read the login redirect request")?;
    if n == 0 {
        return Ok(None);
    }
    let head = String::from_utf8_lossy(&buf[..n]);
    let first_line = head.lines().next().unwrap_or("");
    // "GET /callback?... HTTP/1.1" — the second whitespace-separated token.
    Ok(first_line.split_whitespace().nth(1).map(str::to_string))
}

/// Write a minimal HTTP response with an HTML body, then close. Best-effort: a
/// browser that already navigated away shouldn't fail the login.
async fn respond(stream: &mut TcpStream, status: u16, body: &str) {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        _ => "Not Found",
    };
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\n\
         Content-Type: text/html; charset=utf-8\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         \r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes()).await;
    let _ = stream.flush().await;
}

const PAGE_OK: &str = "<!doctype html><html><head><meta charset=\"utf-8\">\
<title>Signed in</title><style>body{font-family:system-ui,sans-serif;max-width:32rem;\
margin:6rem auto;padding:0 1.5rem;text-align:center;color:#1a1a1a}h1{font-size:1.4rem}\
p{color:#555}</style></head><body><h1>You're signed in \u{2705}</h1>\
<p>You can close this tab and return to the terminal.</p></body></html>";

const PAGE_ERROR: &str = "<!doctype html><html><head><meta charset=\"utf-8\">\
<title>Sign-in error</title><style>body{font-family:system-ui,sans-serif;max-width:32rem;\
margin:6rem auto;padding:0 1.5rem;text-align:center;color:#1a1a1a}h1{font-size:1.4rem}\
p{color:#555}</style></head><body><h1>Sign-in failed</h1>\
<p>Something went wrong. Return to the terminal and try again.</p></body></html>";

const PAGE_DENIED: &str = "<!doctype html><html><head><meta charset=\"utf-8\">\
<title>Sign-in cancelled</title><style>body{font-family:system-ui,sans-serif;max-width:32rem;\
margin:6rem auto;padding:0 1.5rem;text-align:center;color:#1a1a1a}h1{font-size:1.4rem}\
p{color:#555}</style></head><body><h1>Sign-in cancelled</h1>\
<p>You can close this tab and return to the terminal.</p></body></html>";
