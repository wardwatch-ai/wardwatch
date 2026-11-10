//! Client for the accounts backend (wardwatch-ai/backend `accounts/`): sign-in,
//! score and groups.
//!
//! Off unless `WARDWATCH_ACCOUNTS_URL` names a server, so the UI keeps showing
//! sample data until the backend is live. The session token stays here, in the
//! OS keychain, and never reaches the web view: the UI asks for API calls by
//! path and gets the JSON back.
//!
//! Sign-in is the usual desktop-app pattern (RFC 8252 with PKCE, RFC 7636): we
//! listen on 127.0.0.1, ask the backend to email a link, and the link's confirm
//! page sends the browser back to us with a one-time code that only our
//! verifier can trade for a session token.

use std::sync::Mutex;
use std::time::Duration;

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use url::Url;

/// Environment variable naming the accounts server, e.g. `https://wardwatch.ai`
/// or `http://127.0.0.1:8787` for a local `wrangler dev`.
pub const URL_VAR: &str = "WARDWATCH_ACCOUNTS_URL";

/// Emitted to the main window whenever signing in or out changes the state.
pub const CHANGED_EVENT: &str = "accounts-changed";

/// Links last 15 minutes; give the person a little longer to press the button.
const SIGN_IN_WAIT: Duration = Duration::from_secs(16 * 60);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const KEYCHAIN_SERVICE: &str = "ai.wardwatch.Wardwatch";

/// The server's address, if one is set and acceptable: HTTPS, or plain HTTP to
/// this machine only (for testing against a local backend).
pub fn server_from_env() -> Option<Url> {
    let raw = std::env::var(URL_VAR).ok()?;
    match parse_server(&raw) {
        Ok(url) => Some(url),
        Err(e) => {
            eprintln!("{URL_VAR} ignored: {e}");
            None
        }
    }
}

fn parse_server(raw: &str) -> Result<Url, String> {
    let url = Url::parse(raw.trim()).map_err(|e| e.to_string())?;
    let local = matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "[::1]"));
    if !(url.scheme() == "https" || (url.scheme() == "http" && local)) {
        return Err("use https (plain http only for 127.0.0.1 or localhost)".into());
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("no user name or password in the address".into());
    }
    if url.path() != "/" || url.query().is_some() || url.fragment().is_some() {
        return Err("give just the server, like https://wardwatch.ai".into());
    }
    Ok(url)
}

/// Where the session token is kept. The keychain in the app; memory in tests.
pub trait TokenStore: Send + Sync {
    fn get(&self) -> Result<Option<String>, String>;
    fn set(&self, token: &str) -> Result<(), String>;
    fn clear(&self) -> Result<(), String>;
}

/// The OS keychain (macOS Keychain, Windows Credential Manager, Secret Service on Linux).
pub struct Keychain(keyring::Entry);

impl Keychain {
    /// One entry per server, so a local test backend never sees the real token.
    pub fn for_server(server: &Url) -> Result<Self, String> {
        let account = format!("accounts:{}", server.origin().ascii_serialization());
        keyring::Entry::new(KEYCHAIN_SERVICE, &account)
            .map(Self)
            .map_err(|e| e.to_string())
    }
}

/// keyring's Secret Service backend drives its own tokio runtime and panics when
/// called from inside ours, so each keychain call runs on a plain thread.
fn off_runtime<T: Send>(f: impl FnOnce() -> T + Send) -> T {
    std::thread::scope(|s| s.spawn(f).join().expect("keychain thread panicked"))
}

impl TokenStore for Keychain {
    fn get(&self) -> Result<Option<String>, String> {
        match off_runtime(|| self.0.get_password()) {
            Ok(token) => Ok(Some(token)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(format!("couldn't read your sign-in from the keychain: {e}")),
        }
    }
    fn set(&self, token: &str) -> Result<(), String> {
        off_runtime(|| self.0.set_password(token))
            .map_err(|e| format!("couldn't save your sign-in to the keychain: {e}"))
    }
    fn clear(&self) -> Result<(), String> {
        match off_runtime(|| self.0.delete_credential()) {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(format!(
                "couldn't remove your sign-in from the keychain: {e}"
            )),
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Status {
    /// A server is set (`WARDWATCH_ACCOUNTS_URL`). Without one the UI shows samples.
    pub configured: bool,
    pub signed_in: bool,
    /// The server's origin, for Settings.
    pub server: Option<String>,
}

/// What `request` hands back: the HTTP status and the JSON body (null if none).
#[derive(Debug, Serialize)]
pub struct Reply {
    pub status: u16,
    pub body: Value,
}

pub struct Accounts {
    server: Option<Url>,
    http: reqwest::Client,
    store: Option<Box<dyn TokenStore>>,
    /// The token as last read from or written to the store, so the keychain is
    /// read once, not on every call. None until the first read.
    token: Mutex<Option<Option<String>>>,
    /// The sign-in waiting for its link, so a new one replaces it.
    pending: Mutex<Option<tauri::async_runtime::JoinHandle<()>>>,
}

impl Accounts {
    pub fn from_env() -> Self {
        let server = server_from_env();
        let store = server.as_ref().and_then(|s| match Keychain::for_server(s) {
            Ok(k) => Some(Box::new(k) as Box<dyn TokenStore>),
            Err(e) => {
                eprintln!("keychain unavailable: {e}");
                None
            }
        });
        Self::new(server, store)
    }

    pub fn new(server: Option<Url>, store: Option<Box<dyn TokenStore>>) -> Self {
        let http = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .user_agent(concat!("Wardwatch/", env!("CARGO_PKG_VERSION")))
            // Never follow a redirect off the server with our token.
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("HTTP client");
        Self {
            server,
            http,
            store,
            token: Mutex::new(None),
            pending: Mutex::new(None),
        }
    }

    pub fn status(&self) -> Status {
        Status {
            configured: self.server.is_some(),
            signed_in: matches!(self.token(), Ok(Some(_))),
            server: self
                .server
                .as_ref()
                .map(|s| s.origin().ascii_serialization()),
        }
    }

    fn server(&self) -> Result<&Url, String> {
        self.server
            .as_ref()
            .ok_or_else(|| "No accounts server is set up yet.".to_string())
    }

    fn store(&self) -> Result<&dyn TokenStore, String> {
        self.store
            .as_deref()
            .ok_or_else(|| "This device's keychain isn't available.".to_string())
    }

    fn token(&self) -> Result<Option<String>, String> {
        let mut cached = self.token.lock().map_err(|e| e.to_string())?;
        if let Some(token) = &*cached {
            return Ok(token.clone());
        }
        let token = self.store()?.get()?;
        *cached = Some(token.clone());
        Ok(token)
    }

    fn save_token(&self, token: &str) -> Result<(), String> {
        let mut cached = self.token.lock().map_err(|e| e.to_string())?;
        self.store()?.set(token)?;
        *cached = Some(Some(token.to_owned()));
        Ok(())
    }

    /// Forgets the token. With `only`, only if it's still that one, so a late 401
    /// for an old session can't sign out a newer one.
    fn drop_token(&self, only: Option<&str>) -> Result<(), String> {
        let mut cached = self.token.lock().map_err(|e| e.to_string())?;
        if let (Some(only), Some(Some(current))) = (only, &*cached)
            && only != current
        {
            return Ok(());
        }
        self.store()?.clear()?;
        *cached = Some(None);
        Ok(())
    }

    /// One API call for the UI, with the session token added. Only the account,
    /// score and group endpoints can be reached. A 401 means the session ended
    /// (signed out elsewhere, or expired), so the token is dropped.
    pub async fn request(
        &self,
        method: &str,
        path: &str,
        body: Option<Value>,
    ) -> Result<Reply, String> {
        let (method, url) = api_call(self.server()?, method, path)?;
        let token = self.token()?.ok_or("Sign in first.")?;
        let mut req = self.http.request(method, url).bearer_auth(&token);
        if let Some(body) = body {
            req = req.json(&body);
        }
        let res = req.send().await.map_err(offline)?;
        let status = res.status().as_u16();
        let text = res.text().await.map_err(offline)?;
        if status == 401 {
            self.drop_token(Some(&token))?;
        }
        let body = if text.is_empty() {
            Value::Null
        } else {
            serde_json::from_str(&text)
                .map_err(|_| format!("The server sent a bad reply ({status})."))?
        };
        Ok(Reply { status, body })
    }

    /// Signs out this device: ends the session on the server (best effort) and
    /// forgets the token.
    pub async fn sign_out(&self) -> Result<(), String> {
        self.cancel_sign_in();
        if let (Ok(server), Ok(Some(token))) = (self.server(), self.token())
            && let Ok(url) = server.join("/auth/sign-out")
        {
            let _ = self.http.post(url).bearer_auth(token).send().await;
        }
        self.drop_token(None)
    }

    pub fn cancel_sign_in(&self) {
        if let Some(task) = self.pending.lock().ok().and_then(|mut p| p.take()) {
            task.abort();
        }
    }

    pub fn set_pending(&self, task: tauri::async_runtime::JoinHandle<()>) {
        if let Ok(mut pending) = self.pending.lock()
            && let Some(old) = pending.replace(task)
        {
            old.abort();
        }
    }

    /// Step 1 of signing in: listens on the loopback address and asks the server
    /// to email `email` a link. Returns the wait for the link (step 2), which the
    /// caller runs in the background.
    pub async fn start_sign_in(&self, email: &str) -> Result<SignIn, String> {
        let server = self.server()?.clone();
        // Check the keychain works before the email goes out, not after its link is used.
        self.token()?;
        let listener = TcpListener::bind(("127.0.0.1", 0))
            .await
            .map_err(|e| format!("couldn't get ready for the sign-in link: {e}"))?;
        let port = listener.local_addr().map_err(|e| e.to_string())?.port();
        let pkce = Pkce::new()?;
        let url = server.join("/auth/start").map_err(|e| e.to_string())?;
        let res = self
            .http
            .post(url)
            .json(&json!({
                "email": email,
                "app": { "port": port, "state": pkce.state, "challenge": pkce.challenge },
            }))
            .send()
            .await
            .map_err(offline)?;
        if !res.status().is_success() {
            return Err(error_message(res).await);
        }
        Ok(SignIn {
            listener,
            pkce,
            server,
        })
    }

    /// Step 2: waits for the browser to come back with the code, trades it for
    /// the session token and saves it.
    pub async fn finish_sign_in(&self, sign_in: SignIn) -> Result<(), String> {
        let SignIn {
            listener,
            pkce,
            server,
        } = sign_in;
        let wait = async {
            loop {
                let (mut stream, _) = listener.accept().await.map_err(|e| e.to_string())?;
                let Some(code) = read_callback(&mut stream, &pkce.state).await else {
                    let _ = stream.write_all(NOT_FOUND.as_bytes()).await;
                    continue;
                };
                let result = self.exchange(&server, &code, &pkce.verifier).await;
                let _ = stream.write_all(page(result.is_ok()).as_bytes()).await;
                let _ = stream.shutdown().await;
                return result;
            }
        };
        tokio::time::timeout(SIGN_IN_WAIT, wait)
            .await
            .map_err(|_| "The sign-in link wasn't used in time. Ask for a new one.".to_string())?
    }

    async fn exchange(&self, server: &Url, code: &str, verifier: &str) -> Result<(), String> {
        #[derive(Deserialize)]
        struct Exchanged {
            session_token: String,
        }
        let url = server
            .join("/auth/app/exchange")
            .map_err(|e| e.to_string())?;
        let res = self
            .http
            .post(url)
            .json(&json!({ "code": code, "verifier": verifier, "device": device_label() }))
            .send()
            .await
            .map_err(offline)?;
        if !res.status().is_success() {
            return Err(error_message(res).await);
        }
        let body: Exchanged = res.json().await.map_err(|e| e.to_string())?;
        self.save_token(&body.session_token)
    }
}

/// A sign-in waiting for its link.
pub struct SignIn {
    listener: TcpListener,
    pkce: Pkce,
    server: Url,
}

struct Pkce {
    verifier: String,
    challenge: String,
    state: String,
}

impl Pkce {
    fn new() -> Result<Self, String> {
        let verifier = random_b64(32)?;
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        Ok(Self {
            verifier,
            challenge,
            state: random_b64(16)?,
        })
    }
}

fn random_b64(bytes: usize) -> Result<String, String> {
    let mut buf = vec![0u8; bytes];
    getrandom::fill(&mut buf).map_err(|e| e.to_string())?;
    Ok(URL_SAFE_NO_PAD.encode(buf))
}

/// Says which kind of device signed in, for the account's session list. Not the
/// computer's name, which often carries the owner's.
fn device_label() -> String {
    let os = match std::env::consts::OS {
        "macos" => "Mac",
        "windows" => "Windows",
        "linux" => "Linux",
        other => other,
    };
    format!("Wardwatch on {os}")
}

/// The API calls the UI makes, and no others: `*` is one id segment.
/// Sign-in, admin, deleting and everything else stay out of reach.
const API_CALLS: &[(&str, &str)] = &[
    ("GET", "/v1/me"),
    ("PATCH", "/v1/me"),
    ("GET", "/v1/score"),
    ("GET", "/v1/groups"),
    ("POST", "/v1/groups"),
    ("POST", "/v1/groups/join"),
    ("GET", "/v1/groups/stake-ideas"),
    ("GET", "/v1/groups/*"),
    ("PATCH", "/v1/groups/*"),
    ("GET", "/v1/groups/*/leaderboard"),
    ("PUT", "/v1/groups/*/stake"),
];

fn api_call(server: &Url, method: &str, path: &str) -> Result<(reqwest::Method, Url), String> {
    let refuse = || format!("{method} {path} isn't an API call the app may make");
    let (route, query) = path.split_once('?').unwrap_or((path, ""));
    let segment_ok = |seg: &str| {
        !seg.is_empty()
            && seg
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
    };
    let matches = |pattern: &str| {
        let (mut want, mut got) = (pattern.split('/'), route.split('/'));
        loop {
            match (want.next(), got.next()) {
                (None, None) => return true,
                (Some("*"), Some(seg)) if segment_ok(seg) => {}
                (Some(w), Some(g)) if w == g => {}
                _ => return false,
            }
        }
    };
    let query_ok = query
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b"=&_-".contains(&b));
    if !query_ok || !API_CALLS.iter().any(|(m, p)| *m == method && matches(p)) {
        return Err(refuse());
    }
    let method = reqwest::Method::from_bytes(method.as_bytes()).map_err(|_| refuse())?;
    Ok((method, server.join(path).map_err(|e| e.to_string())?))
}

fn offline(e: reqwest::Error) -> String {
    if e.is_timeout() || e.is_connect() {
        "Couldn't reach Wardwatch. Check your connection and try again.".into()
    } else {
        format!("Couldn't reach Wardwatch: {e}")
    }
}

/// The backend's `{error: {message}}`, or a plain fallback.
async fn error_message(res: reqwest::Response) -> String {
    let status = res.status();
    res.json::<Value>()
        .await
        .ok()
        .and_then(|v| v["error"]["message"].as_str().map(str::to_owned))
        .unwrap_or_else(|| format!("The server said no ({status})."))
}

/// Reads one HTTP request from the browser. Returns the code if it's
/// `GET /callback?code=…&state=<ours>`; anything else (a favicon request, a
/// stray visit, the wrong state) gets None.
async fn read_callback(stream: &mut tokio::net::TcpStream, state: &str) -> Option<String> {
    let mut buf = Vec::with_capacity(1024);
    let mut chunk = [0u8; 1024];
    while !buf.windows(4).any(|w| w == b"\r\n\r\n") && buf.len() < 8192 {
        let n = tokio::time::timeout(Duration::from_secs(2), stream.read(&mut chunk))
            .await
            .ok()?
            .ok()?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
    }
    let line = std::str::from_utf8(&buf).ok()?.lines().next()?;
    parse_callback(line, state)
}

fn parse_callback(request_line: &str, state: &str) -> Option<String> {
    let mut parts = request_line.split(' ');
    let (method, target) = (parts.next()?, parts.next()?);
    if method != "GET" {
        return None;
    }
    let url = Url::parse("http://127.0.0.1").ok()?.join(target).ok()?;
    if url.path() != "/callback" {
        return None;
    }
    let (mut code, mut ok) = (None, false);
    for (k, v) in url.query_pairs() {
        match &*k {
            "code" => code = Some(v.into_owned()),
            "state" => ok = v == state,
            _ => {}
        }
    }
    code.filter(|c| ok && !c.is_empty() && c.bytes().all(|b| b.is_ascii_alphanumeric()))
}

/// The page the browser shows after coming back to us.
fn page(ok: bool) -> String {
    let (status, title, text) = if ok {
        (
            "200 OK",
            "You're signed in",
            "Go back to Wardwatch. You can close this tab.",
        )
    } else {
        (
            "400 Bad Request",
            "That didn't work",
            "Go back to Wardwatch and ask for a new link.",
        )
    };
    let body = format!(
        "<!doctype html><meta charset=utf-8><meta name=viewport content=\"width=device-width\">\
         <title>Wardwatch</title><body style=\"font:16px/1.5 system-ui,sans-serif;max-width:420px;\
         margin:15vh auto;padding:16px;text-align:center\"><h1>{title}</h1><p>{text}</p>"
    );
    format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\n\
         Cache-Control: no-store\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

const NOT_FOUND: &str = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn servers_must_be_https_or_this_machine() {
        assert!(parse_server("https://wardwatch.ai").is_ok());
        assert!(parse_server("http://127.0.0.1:8787").is_ok());
        assert!(parse_server("http://localhost:8787/").is_ok());
        assert!(parse_server("http://wardwatch.ai").is_err());
        assert!(parse_server("https://wardwatch.ai/v1").is_err());
        assert!(parse_server("https://user:pw@wardwatch.ai").is_err());
        assert!(parse_server("ftp://wardwatch.ai").is_err());
    }

    #[test]
    fn only_the_calls_the_ui_makes() {
        let s = Url::parse("https://wardwatch.ai").unwrap();
        for (m, ok) in [
            ("GET", "/v1/me"),
            ("PATCH", "/v1/me"),
            ("GET", "/v1/score"),
            ("POST", "/v1/groups/join"),
            ("GET", "/v1/groups/g_abc/leaderboard?period=30d"),
            ("PUT", "/v1/groups/g_abc/stake"),
        ] {
            let (method, url) = api_call(&s, m, ok).unwrap();
            assert_eq!(method.as_str(), m);
            assert_eq!(url.as_str(), format!("https://wardwatch.ai{ok}"));
        }
        for (m, bad) in [
            ("GET", "/admin"),
            ("POST", "/auth/start"),
            ("GET", "/auth/session"),
            ("GET", "/auth/token?aud=admin"),
            ("DELETE", "/v1/groups/g_abc"),
            ("POST", "/v1/score/patches"),
            ("GET", "/v1/users/u_abc"),
            ("GET", "/v1/score/../../admin"),
            ("GET", "/v1/meow"),
            ("GET", "/v1/groups/"),
            ("GET", "//evil.example/v1/me"),
            ("GET", "https://evil.example/v1/me"),
            ("GET", "/v1/groups?x=%2F"),
            ("GET", "/v1/groups/a b"),
            ("get", "/v1/me"),
        ] {
            assert!(api_call(&s, m, bad).is_err(), "{m} {bad} should be refused");
        }
    }

    #[test]
    fn a_late_401_only_drops_the_token_it_was_for() {
        let a = Accounts::new(None, Some(Box::<Memory>::default()));
        a.save_token("new").unwrap();
        a.drop_token(Some("old")).unwrap();
        assert_eq!(a.token().unwrap().as_deref(), Some("new"));
        a.drop_token(Some("new")).unwrap();
        assert_eq!(a.token().unwrap(), None);
    }

    #[test]
    fn pkce_challenge_is_s256_of_the_verifier() {
        let p = Pkce::new().unwrap();
        assert_eq!(p.verifier.len(), 43);
        assert_eq!(p.challenge.len(), 43);
        assert!(p.state.len() >= 16);
        assert_eq!(
            p.challenge,
            URL_SAFE_NO_PAD.encode(Sha256::digest(p.verifier.as_bytes()))
        );
        // RFC 7636 appendix B.
        assert_eq!(
            URL_SAFE_NO_PAD.encode(Sha256::digest(
                b"dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"
            )),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn callback_needs_our_state_and_path() {
        let st = "state-abcdefghijkl";
        assert_eq!(
            parse_callback(
                &format!("GET /callback?code=abc123&state={st} HTTP/1.1"),
                st
            ),
            Some("abc123".into())
        );
        assert_eq!(
            parse_callback("GET /callback?code=abc123&state=other HTTP/1.1", st),
            None
        );
        assert_eq!(
            parse_callback(&format!("GET /favicon.ico?state={st} HTTP/1.1"), st),
            None
        );
        assert_eq!(
            parse_callback(&format!("POST /callback?code=a&state={st} HTTP/1.1"), st),
            None
        );
        assert_eq!(
            parse_callback(&format!("GET /callback?code=a%3Cb&state={st} HTTP/1.1"), st),
            None
        );
    }

    #[derive(Default)]
    struct Memory(Mutex<Option<String>>);
    impl TokenStore for Memory {
        fn get(&self) -> Result<Option<String>, String> {
            Ok(self.0.lock().unwrap().clone())
        }
        fn set(&self, t: &str) -> Result<(), String> {
            *self.0.lock().unwrap() = Some(t.into());
            Ok(())
        }
        fn clear(&self) -> Result<(), String> {
            *self.0.lock().unwrap() = None;
            Ok(())
        }
    }

    #[test]
    fn without_a_server_it_is_off() {
        let a = Accounts::new(None, Some(Box::<Memory>::default()));
        assert_eq!(
            a.status(),
            Status {
                configured: false,
                signed_in: false,
                server: None
            }
        );
    }

    /// End to end against a local backend (`npm run dev` in wardwatch-ai/backend
    /// accounts/, which prints sign-in emails to its log). Run with
    /// `WARDWATCH_ACCOUNTS_URL=http://127.0.0.1:8787 WARDWATCH_EMAIL_LOG=<wrangler log>
    /// cargo test -p wardwatch-desktop accounts -- --ignored`. It plays the browser:
    /// opens the emailed link, presses the button and follows the redirect home.
    #[tokio::test]
    #[ignore = "needs a local accounts backend"]
    async fn signs_in_against_a_local_backend() {
        let server = server_from_env().expect("set WARDWATCH_ACCOUNTS_URL");
        let log = std::env::var("WARDWATCH_EMAIL_LOG").expect("set WARDWATCH_EMAIL_LOG");
        let email = format!("e2e-{}@example.com", random_b64(6).unwrap().to_lowercase());
        let a = Accounts::new(Some(server.clone()), Some(Box::<Memory>::default()));
        assert!(
            a.request("GET", "/v1/me", None).await.is_err(),
            "signed out first"
        );

        let sign_in = a.start_sign_in(&email).await.unwrap();
        let link = wait_for_link(&log, &email).await;
        let browser = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();
        let confirm = browser
            .get(&link)
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        let field = |name: &str| {
            let at = confirm.find(&format!("name=\"{name}\" value=\"")).unwrap() + name.len() + 15;
            confirm[at..at + confirm[at..].find('"').unwrap()].to_string()
        };
        let redeem = browser
            .post(server.join("/auth/redeem").unwrap())
            .header("Origin", server.origin().ascii_serialization())
            .form(&[("t", field("t")), ("app", field("app"))])
            .send()
            .await
            .unwrap();
        assert_eq!(redeem.status(), 303);
        let home = redeem.headers()["location"].to_str().unwrap().to_string();
        assert!(home.starts_with("http://127.0.0.1:"), "{home}");
        let (finish, page) = tokio::join!(a.finish_sign_in(sign_in), async {
            browser
                .get(&home)
                .send()
                .await
                .unwrap()
                .text()
                .await
                .unwrap()
        });
        finish.unwrap();
        assert!(page.contains("You're signed in"));
        assert!(a.status().signed_in);

        let me = a.request("GET", "/v1/me", None).await.unwrap();
        assert_eq!(me.status, 200);
        assert_eq!(me.body["email"], email.as_str());
        let named = a
            .request(
                "PATCH",
                "/v1/me",
                Some(json!({ "username": format!("e2e{}", &email[4..10]) })),
            )
            .await
            .unwrap();
        assert_eq!(named.status, 200, "{}", named.body);
        let group = a
            .request("POST", "/v1/groups", Some(json!({ "name": "Family" })))
            .await
            .unwrap();
        assert_eq!(group.status, 201, "{}", group.body);
        let score = a.request("GET", "/v1/score", None).await.unwrap();
        assert_eq!(score.status, 200);
        assert_eq!(score.body["patches_scored"], 0);

        a.sign_out().await.unwrap();
        assert!(!a.status().signed_in);
    }

    async fn wait_for_link(log: &str, email: &str) -> String {
        for _ in 0..50 {
            let text = std::fs::read_to_string(log).unwrap_or_default();
            if let Some(at) = text.rfind(email)
                && let Some(start) = text[at..].find("/auth/confirm?")
            {
                let begin = text[..at + start].rfind("http").unwrap();
                let end = text[begin..]
                    .find(|c: char| c.is_whitespace() || c == '"')
                    .map_or(text.len(), |e| begin + e);
                return text[begin..end].to_string();
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        panic!("no sign-in email for {email} in {log}");
    }
}
