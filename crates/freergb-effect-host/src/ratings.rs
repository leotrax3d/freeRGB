//! Shared ratings using Supabase anonymous authentication. No registration UI,
//! GitHub identity, device fingerprint or hardware information is sent here.
use reqwest::blocking::{Client as Http, Response};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    path::PathBuf,
    sync::Mutex,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Summary {
    pub average: f64,
    pub count: u64,
}
#[derive(Debug, Clone, Default, Serialize)]
pub struct Ratings {
    pub configured: bool,
    pub by_effect: BTreeMap<String, Summary>,
    pub mine: BTreeMap<String, u8>,
    pub updated_at: Option<u64>,
    pub loading: bool,
    pub saving: Option<String>,
    pub error: Option<String>,
}
struct Config {
    url: String,
    key: String,
}
#[derive(Clone, Serialize, Deserialize)]
struct Session {
    access_token: String,
    refresh_token: String,
    user: User,
    #[serde(default)]
    expires_at: u64,
}
#[derive(Clone, Serialize, Deserialize)]
struct User {
    id: String,
}
pub struct Client {
    config: Config,
    http: Http,
    file: PathBuf,
    session: Mutex<Option<Session>>,
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn safe_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        && !s.contains("..")
}
fn response(response: Response) -> Result<Value, String> {
    let status = response.status();
    if !status.is_success() {
        return Err(format!(
            "Community service request failed (HTTP {}). Please retry later.",
            status.as_u16()
        ));
    }
    let mut bytes = Vec::new();
    response
        .take(128 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Could not read community response")?;
    if bytes.len() > 128 * 1024 {
        return Err("Community response exceeds size limit".into());
    }
    if bytes.is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_slice(&bytes).map_err(|_| "Invalid community service response".into())
}
impl Client {
    pub fn configured(file: PathBuf) -> Result<Option<Self>, String> {
        let url = std::env::var("FREERGB_SUPABASE_URL")
            .ok()
            .or_else(|| option_env!("FREERGB_SUPABASE_URL").map(str::to_owned))
            .filter(|v| !v.is_empty());
        let key = std::env::var("FREERGB_SUPABASE_PUBLISHABLE_KEY")
            .ok()
            .or_else(|| option_env!("FREERGB_SUPABASE_PUBLISHABLE_KEY").map(str::to_owned))
            .filter(|v| !v.is_empty());
        match (url, key) {
            (None, None) => Ok(None),
            (Some(url), Some(key)) => {
                use sha2::{Digest, Sha256};
                let namespace = hex::encode(Sha256::digest(url.trim_end_matches('/').as_bytes()));
                let file =
                    file.with_file_name(format!("ratings-session-{}.json", &namespace[..16]));
                let development = cfg!(debug_assertions)
                    && std::env::var("FREERGB_SUPABASE_DEV_HTTP").as_deref() == Ok("1");
                Self::new(url, key, file, development).map(Some)
            }
            _ => Err("Community ratings need a Supabase project URL and publishable key".into()),
        }
    }
    fn new(url: String, key: String, file: PathBuf, development: bool) -> Result<Self, String> {
        let parsed = reqwest::Url::parse(&url).map_err(|_| "Invalid community service URL")?;
        let local = development
            && parsed.scheme() == "http"
            && matches!(parsed.host_str(), Some("127.0.0.1" | "localhost"));
        if (parsed.scheme() != "https" && !local)
            || parsed.host_str().is_none()
            || !parsed.username().is_empty()
            || parsed.password().is_some()
            || parsed.query().is_some()
            || parsed.fragment().is_some()
            || parsed.path() != "/"
        {
            return Err(
                "Community service needs an HTTPS origin without credentials, path or query".into(),
            );
        }
        // Only public/publishable keys belong in distributed desktop builds.
        if key.is_empty() || key.len() > 4096 || key.starts_with("sb_secret_") {
            return Err("Use a Supabase publishable key, never a secret/service-role key".into());
        }
        if !key.starts_with("sb_publishable_") {
            use base64::Engine;
            let role = key
                .split('.')
                .nth(1)
                .and_then(|part| {
                    base64::engine::general_purpose::URL_SAFE_NO_PAD
                        .decode(part)
                        .ok()
                })
                .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
                .and_then(|v| v["role"].as_str().map(str::to_owned));
            if role.as_deref() != Some("anon") {
                return Err(
                    "Use a Supabase publishable key or legacy anon key, never a service-role key"
                        .into(),
                );
            }
        }
        let http = Http::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(15))
            .redirect(reqwest::redirect::Policy::none())
            .user_agent(concat!("freeRGB/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|_| "Could not initialize community client")?;
        Ok(Self {
            config: Config {
                url: url.trim_end_matches('/').into(),
                key,
            },
            http,
            file,
            session: Mutex::new(None),
        })
    }
    fn request(&self, method: reqwest::Method, path: &str) -> reqwest::blocking::RequestBuilder {
        self.http
            .request(method, format!("{}{path}", self.config.url))
            .header("apikey", &self.config.key)
            .header("Content-Type", "application/json")
    }
    fn session(&self, create: bool, force: bool) -> Result<Option<Session>, String> {
        let mut cached = self.session.lock().unwrap_or_else(|e| e.into_inner());
        if cached.is_none() && self.file.exists() {
            let f = std::fs::File::open(&self.file)
                .map_err(|_| "Could not read anonymous rating identity")?;
            let mut bytes = Vec::new();
            f.take(64 * 1024 + 1)
                .read_to_end(&mut bytes)
                .map_err(|_| "Could not read anonymous rating identity")?;
            if bytes.len() > 64 * 1024 {
                return Err("Invalid anonymous rating identity".into());
            }
            *cached =
                Some(serde_json::from_slice(&bytes).map_err(|_| {
                    "Invalid anonymous rating identity; no new identity was created"
                })?);
        }
        let current = cached.clone();
        if current.is_none() && !create {
            return Ok(None);
        }
        if let Some(s) = &current {
            Self::validate_session(s)?;
            if !force && s.expires_at > now() + 60 {
                return Ok(current);
            }
        }
        let (path, body) = match &current {
            Some(s) => (
                "/auth/v1/token?grant_type=refresh_token",
                json!({"refresh_token":s.refresh_token}),
            ),
            None => ("/auth/v1/signup", json!({})),
        };
        let value = response(
            self.request(reqwest::Method::POST, path)
                .body(body.to_string())
                .send()
                .map_err(|_| "Anonymous ratings are temporarily unavailable")?,
        )?;
        let expires_in = value["expires_in"]
            .as_u64()
            .filter(|v| *v > 0 && *v <= 86400);
        let mut session: Session = serde_json::from_value(value)
            .map_err(|_| "Anonymous sign-in is not enabled or the session response is invalid")?;
        if session.expires_at == 0 {
            session.expires_at =
                now() + expires_in.ok_or("Anonymous session has no valid expiry")?;
        }
        Self::validate_session(&session)?;
        if current
            .as_ref()
            .is_some_and(|s| s.user.id != session.user.id)
        {
            return Err("Anonymous identity changed unexpectedly".into());
        }
        // Keep JWT expiry away from the next write's refresh boundary.
        if session.expires_at <= now() {
            return Err("Community service returned an expired session".into());
        }
        session.user.id.make_ascii_lowercase();
        self.save(&session)?;
        *cached = Some(session.clone());
        Ok(Some(session))
    }
    fn validate_session(s: &Session) -> Result<(), String> {
        if s.access_token.is_empty()
            || s.refresh_token.is_empty()
            || s.access_token.len() > 16384
            || s.refresh_token.len() > 16384
            || s.user.id.len() != 36
            || !s
                .user
                .id
                .bytes()
                .all(|b| b.is_ascii_hexdigit() || b == b'-')
        {
            return Err("Invalid anonymous rating identity".into());
        }
        Ok(())
    }
    fn save(&self, session: &Session) -> Result<(), String> {
        let parent = self.file.parent().ok_or("No rating identity directory")?;
        std::fs::create_dir_all(parent).map_err(|_| "Cannot save anonymous rating identity")?;
        let temp = self.file.with_extension(format!(
            "{}-{}.tmp",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let result = (|| {
            let mut f = options
                .open(&temp)
                .map_err(|_| "Cannot save anonymous rating identity")?;
            let bytes = serde_json::to_vec(session)
                .map_err(|_| "Cannot encode anonymous rating identity")?;
            f.write_all(&bytes)
                .and_then(|()| f.sync_all())
                .map_err(|_| "Cannot save anonymous rating identity")?;
            drop(f);
            std::fs::rename(&temp, &self.file)
                .map_err(|_| "Cannot save anonymous rating identity")?;
            Ok(())
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(temp);
        }
        result
    }
    pub fn fetch(&self) -> Result<Ratings, String> {
        let data = response(
            self.request(reqwest::Method::POST, "/rest/v1/rpc/effect_rating_summary")
                .body("{}")
                .send()
                .map_err(|_| "Community ratings are temporarily unavailable")?,
        )?;
        let mut ratings = Ratings {
            configured: true,
            updated_at: Some(now()),
            ..Default::default()
        };
        for row in data.as_array().ok_or("Invalid rating summary")? {
            let id = row["effect_id"]
                .as_str()
                .ok_or("Invalid effect id in rating summary")?;
            let average = row["average"].as_f64().ok_or("Invalid rating average")?;
            let count = row["count"].as_u64().ok_or("Invalid rating count")?;
            if !safe_id(id)
                || !average.is_finite()
                || !(1.0..=5.0).contains(&average)
                || count == 0
                || ratings.by_effect.len() >= 128
                || ratings.by_effect.contains_key(id)
            {
                return Err("Invalid rating summary".into());
            }
            ratings
                .by_effect
                .insert(id.into(), Summary { average, count });
        }
        if let Some(session) = self.session(false, false)? {
            let data = response(
                self.request(reqwest::Method::GET, "/rest/v1/effect_ratings")
                    .bearer_auth(&session.access_token)
                    .query(&[
                        ("select", "effect_id,stars"),
                        ("user_id", &format!("eq.{}", session.user.id)),
                    ])
                    .send()
                    .map_err(|_| "Could not read your ratings")?,
            )?;
            for row in data.as_array().ok_or("Invalid personal rating response")? {
                let id = row["effect_id"].as_str().ok_or("Invalid rated effect")?;
                let stars = row["stars"].as_u64().ok_or("Invalid personal rating")?;
                if !safe_id(id) || !(1..=5).contains(&stars) || ratings.mine.len() >= 128 {
                    return Err("Invalid personal rating response".into());
                }
                ratings.mine.insert(id.into(), stars as u8);
            }
        }
        Ok(ratings)
    }
    pub fn rate(&self, id: &str, stars: Option<u8>) -> Result<(), String> {
        if !safe_id(id) || stars.is_some_and(|s| !(1..=5).contains(&s)) {
            return Err("Choose between one and five stars".into());
        }
        let Some(mut session) = self.session(stars.is_some(), false)? else {
            return Ok(());
        };
        for attempt in 0..2 {
            let request = match stars {
                Some(stars) => self
                    .request(reqwest::Method::POST, "/rest/v1/effect_ratings")
                    .query(&[("on_conflict", "user_id,effect_id")])
                    .header("Prefer", "resolution=merge-duplicates,return=minimal")
                    .body(
                        json!({"user_id":session.user.id,"effect_id":id,"stars":stars}).to_string(),
                    ),
                None => self
                    .request(reqwest::Method::DELETE, "/rest/v1/effect_ratings")
                    .query(&[
                        ("effect_id", format!("eq.{id}")),
                        ("user_id", format!("eq.{}", session.user.id)),
                    ]),
            };
            let reply = request
                .bearer_auth(&session.access_token)
                .send()
                .map_err(|_| "Could not save your rating; please retry")?;
            if reply.status() == reqwest::StatusCode::UNAUTHORIZED && attempt == 0 {
                session = self
                    .session(true, true)?
                    .ok_or("Anonymous identity unavailable")?;
                continue;
            }
            response(reply)?;
            return Ok(());
        }
        Err("Could not save your rating".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicUsize, Ordering};
    const USER: &str = "12345678-1234-1234-1234-123456789abc";
    type Step = (&'static str, u16, Value);
    fn mock(steps: Vec<Step>) -> (String, std::thread::JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        let handle = std::thread::spawn(move || {
            let mut requests = Vec::new();
            for (expected, status, body) in steps {
                let (mut socket, _) = listener.accept().unwrap();
                socket
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                let mut bytes = Vec::new();
                let header_end;
                loop {
                    let mut byte = [0];
                    socket.read_exact(&mut byte).unwrap();
                    bytes.push(byte[0]);
                    if bytes.ends_with(b"\r\n\r\n") {
                        header_end = bytes.len();
                        break;
                    }
                }
                let headers = String::from_utf8(bytes.clone()).unwrap();
                let length = headers
                    .lines()
                    .find_map(|line| {
                        line.to_lowercase()
                            .strip_prefix("content-length: ")
                            .and_then(|n| n.trim().parse::<usize>().ok())
                    })
                    .unwrap_or(0);
                bytes.resize(header_end + length, 0);
                socket.read_exact(&mut bytes[header_end..]).unwrap();
                let request = String::from_utf8(bytes).unwrap();
                assert!(request.starts_with(expected), "unexpected request route");
                assert!(request
                    .to_lowercase()
                    .contains("apikey: sb_publishable_test"));
                requests.push(request);
                let body = if status == 204 {
                    String::new()
                } else {
                    body.to_string()
                };
                write!(socket,"HTTP/1.1 {status} response\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
            }
            requests
        });
        (address, handle)
    }
    fn path() -> PathBuf {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        std::env::temp_dir().join(format!(
            "freergb-ratings-{}-{}.json",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ))
    }
    fn session(token: &str, expiry: u64) -> Value {
        json!({"access_token":token,"refresh_token":"test-refresh-token","user":{"id":USER},"expires_at":expiry})
    }
    #[test]
    fn anonymous_rating_persists_identity_updates_and_reads_real_totals() {
        let (url, server) = mock(vec![
            (
                "POST /auth/v1/signup",
                200,
                session("test-access-token", now() + 3600),
            ),
            ("POST /rest/v1/effect_ratings?", 201, Value::Null),
            ("POST /rest/v1/effect_ratings?", 201, Value::Null),
            (
                "POST /rest/v1/rpc/effect_rating_summary",
                200,
                json!([{"effect_id":"aurora","average":4.0,"count":2}]),
            ),
            (
                "GET /rest/v1/effect_ratings?",
                200,
                json!([{"effect_id":"aurora","stars":3}]),
            ),
            ("DELETE /rest/v1/effect_ratings?", 204, Value::Null),
        ]);
        let file = path();
        let c = Client::new(
            url.clone(),
            "sb_publishable_test".into(),
            file.clone(),
            true,
        )
        .unwrap();
        c.rate("aurora", Some(5)).unwrap();
        drop(c);
        let c = Client::new(url, "sb_publishable_test".into(), file.clone(), true).unwrap();
        c.rate("aurora", Some(3)).unwrap();
        let ratings = c.fetch().unwrap();
        assert_eq!(ratings.mine["aurora"], 3);
        assert_eq!(
            ratings.by_effect["aurora"],
            Summary {
                average: 4.0,
                count: 2
            }
        );
        c.rate("aurora", None).unwrap();
        let requests = server.join().unwrap();
        for request in &requests[1..3] {
            assert!(request
                .to_lowercase()
                .contains("authorization: bearer test-access-token"));
            let value: Value =
                serde_json::from_str(request.split("\r\n\r\n").nth(1).unwrap()).unwrap();
            assert_eq!(value["user_id"], USER);
            assert_eq!(value["effect_id"], "aurora");
            assert_eq!(
                value.as_object().unwrap().len(),
                3,
                "no hardware or device identity is submitted"
            );
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&file).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        let _ = std::fs::remove_file(file);
    }
    #[test]
    fn refreshes_expired_anonymous_session_without_creating_a_new_identity() {
        let (url, server) = mock(vec![
            (
                "POST /auth/v1/signup",
                200,
                session("old-token", now() + 20),
            ),
            ("POST /rest/v1/effect_ratings?", 201, Value::Null),
            (
                "POST /auth/v1/token?grant_type=refresh_token",
                200,
                session("new-token", now() + 3600),
            ),
            ("POST /rest/v1/effect_ratings?", 201, Value::Null),
        ]);
        let file = path();
        let c = Client::new(url, "sb_publishable_test".into(), file.clone(), true).unwrap();
        c.rate("aurora", Some(2)).unwrap();
        c.rate("aurora", Some(4)).unwrap();
        let requests = server.join().unwrap();
        assert!(requests[2].contains("test-refresh-token"));
        assert!(requests[3]
            .to_lowercase()
            .contains("authorization: bearer new-token"));
        let _ = std::fs::remove_file(file);
    }
    #[test]
    fn never_treats_a_rejected_vote_as_saved_and_rejects_unsafe_configuration() {
        let (url, server) = mock(vec![
            ("POST /auth/v1/signup", 200, session("token", now() + 3600)),
            (
                "POST /rest/v1/effect_ratings?",
                403,
                json!({"message":"RLS denied"}),
            ),
        ]);
        let file = path();
        let c = Client::new(url, "sb_publishable_test".into(), file.clone(), true).unwrap();
        assert!(c.rate("aurora", Some(5)).is_err());
        assert!(c.rate("aurora", Some(6)).is_err());
        server.join().unwrap();
        assert!(Client::new(
            "http://evil.example".into(),
            "sb_publishable_test".into(),
            path(),
            false
        )
        .is_err());
        assert!(Client::new(
            "https://project.supabase.co".into(),
            "sb_secret_private".into(),
            path(),
            false
        )
        .is_err());
        use base64::Engine;
        let encoded =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(br#"{"role":"service_role"}"#);
        assert!(Client::new(
            "https://project.supabase.co".into(),
            format!("x.{encoded}.x"),
            path(),
            false
        )
        .is_err());
        let _ = std::fs::remove_file(file);
    }
}
