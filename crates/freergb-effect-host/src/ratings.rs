//! Appwrite community client. Only a random anonymous identity is used for votes.
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
    pub downloads: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    #[test]
    fn native_session_survives_restart_and_receipts_retry_without_new_identity() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/v1", listener.local_addr().unwrap());
        let file = std::env::temp_dir().join(format!(
            "freergb-appwrite-test-{}.json",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let server = std::thread::spawn(move || {
            let replies = [
                (
                    "/account/sessions/anonymous",
                    200,
                    json!({"userId":"test-user"}),
                    true,
                ),
                ("/account/jwts", 200, json!({"jwt":"test-jwt"}), false),
                (
                    "/functions/community-api/executions",
                    200,
                    json!({"responseStatusCode":503,"responseBody":"{}"}),
                    false,
                ),
                ("/account/jwts", 200, json!({"jwt":"test-jwt"}), false),
                (
                    "/functions/community-api/executions",
                    200,
                    json!({"responseStatusCode":200,"responseBody":"{\"counted\":false}"}),
                    false,
                ),
                (
                    "/functions/community-api/executions",
                    200,
                    json!({"responseStatusCode":200,"responseBody":"{\"by_effect\":{\"aurora\":{\"average\":4,\"count\":2,\"downloads\":1}}}"}),
                    false,
                ),
                ("/account/jwts", 200, json!({"jwt":"test-jwt"}), false),
                (
                    "/functions/community-api/executions",
                    200,
                    json!({"responseStatusCode":200,"responseBody":"{\"mine\":{\"aurora\":5}}"}),
                    false,
                ),
            ];
            let mut actions = Vec::new();
            for (path, status, body, cookie) in replies {
                let (mut socket, _) = listener.accept().unwrap();
                socket
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                let mut bytes = Vec::new();
                loop {
                    let mut byte = [0];
                    socket.read_exact(&mut byte).unwrap();
                    bytes.push(byte[0]);
                    if bytes.ends_with(b"\r\n\r\n") {
                        break;
                    }
                }
                let headers = String::from_utf8(bytes).unwrap();
                assert!(headers.starts_with(&format!("POST /v1{path}")));
                assert!(headers
                    .to_lowercase()
                    .contains("x-appwrite-project: test-project"));
                if path == "/account/jwts" {
                    assert!(headers.contains("a_session_test-project=test-cookie"));
                }
                let len = headers
                    .lines()
                    .find_map(|s| {
                        s.to_lowercase()
                            .strip_prefix("content-length: ")
                            .and_then(|n| n.parse::<usize>().ok())
                    })
                    .unwrap();
                let mut body_bytes = vec![0; len];
                socket.read_exact(&mut body_bytes).unwrap();
                if path.ends_with("executions") {
                    let execution: Value = serde_json::from_slice(&body_bytes).unwrap();
                    let payload: Value =
                        serde_json::from_str(execution["body"].as_str().unwrap()).unwrap();
                    if payload["action"] != "summary" {
                        assert_eq!(execution["headers"]["x-freergb-jwt"], "test-jwt");
                    }
                    actions.push(payload["action"].as_str().unwrap().to_owned());
                }
                let body = body.to_string();
                let cookie = if cookie {
                    "Set-Cookie: a_session_test-project=test-cookie; HttpOnly; Path=/\r\n"
                } else {
                    ""
                };
                write!(socket,"HTTP/1.1 {status} OK\r\nContent-Type: application/json\r\n{cookie}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
            }
            actions
        });
        let client = Client::new(url.clone(), "test-project".into(), file.clone(), true).unwrap();
        assert!(client.download("aurora", "1.0.0").is_err());
        drop(client);
        let client = Client::new(url, "test-project".into(), file.clone(), true).unwrap();
        let ratings = client.fetch(&["aurora".into()]).unwrap();
        assert_eq!(ratings.by_effect["aurora"].downloads, 1);
        assert_eq!(ratings.mine["aurora"], 5);
        assert_eq!(
            server.join().unwrap(),
            ["download", "download", "summary", "mine"]
        );
        assert_eq!(
            std::fs::read_dir(file.with_extension("downloads"))
                .unwrap()
                .count(),
            0
        );
        std::fs::remove_file(&file).unwrap();
        std::fs::remove_dir(file.with_extension("downloads")).unwrap();
    }
}
/// Persisted user choice for the online community service (ratings, shared
/// download counts, reviewed reports). Both switches default to off: until the
/// user opts in, no request reaches the community service and no anonymous
/// identity or download receipt is created.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct CommunitySettings {
    /// Allow contact with the community service at all.
    pub online: bool,
    /// Additionally allow sending a download receipt after an install.
    /// Only effective together with `online`.
    pub count_downloads: bool,
}
impl CommunitySettings {
    pub fn counting(&self) -> bool {
        self.online && self.count_downloads
    }
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
#[derive(Clone, Serialize, Deserialize)]
struct Session {
    cookie: String,
}
pub struct Client {
    url: String,
    project: String,
    http: Http,
    file: PathBuf,
    session: Mutex<Option<Session>>,
    outbox: Mutex<()>,
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn safe_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        && !s.contains("..")
}
fn response(reply: Response) -> Result<Value, String> {
    let status = reply.status();
    if !status.is_success() {
        return Err(format!(
            "Community request failed (HTTP {}). Please retry; your identity was retained.",
            status.as_u16()
        ));
    }
    let mut bytes = Vec::new();
    reply
        .take(128 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Cannot read community response")?;
    if bytes.len() > 128 * 1024 {
        return Err("Community response exceeds size limit".into());
    }
    serde_json::from_slice(&bytes).map_err(|_| "Invalid community response".into())
}
impl Client {
    pub fn configured(file: PathBuf) -> Result<Option<Self>, String> {
        if cfg!(test) {
            return Ok(None);
        }
        let url = std::env::var("FREERGB_APPWRITE_ENDPOINT")
            .unwrap_or_else(|_| "https://fra.cloud.appwrite.io/v1".into());
        let project = std::env::var("FREERGB_APPWRITE_PROJECT_ID")
            .unwrap_or_else(|_| "6adb5259001299138e7b".into());
        if url.is_empty() && project.is_empty() {
            return Ok(None);
        }
        use sha2::{Digest, Sha256};
        let namespace = hex::encode(Sha256::digest(format!("{url}/{project}")));
        let file = file.with_file_name(format!("appwrite-session-{}.json", &namespace[..16]));
        Self::new(url, project, file, false).map(Some)
    }
    pub(crate) fn new(
        url: String,
        project: String,
        file: PathBuf,
        development: bool,
    ) -> Result<Self, String> {
        let parsed = reqwest::Url::parse(&url).map_err(|_| "Invalid Appwrite endpoint")?;
        let local = development
            && parsed.scheme() == "http"
            && matches!(parsed.host_str(), Some("127.0.0.1" | "localhost"));
        if (parsed.scheme() != "https" && !local)
            || parsed.host_str().is_none()
            || !parsed.username().is_empty()
            || parsed.password().is_some()
            || parsed.query().is_some()
            || parsed.fragment().is_some()
            || parsed.path() != "/v1"
            || !safe_id(&project)
        {
            return Err("Appwrite needs an HTTPS /v1 endpoint and public project ID".into());
        }
        let http = Http::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(45))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| "Cannot initialize community client")?;
        Ok(Self {
            url,
            project,
            http,
            file,
            session: Mutex::new(None),
            outbox: Mutex::new(()),
        })
    }
    fn request(&self, path: &str) -> reqwest::blocking::RequestBuilder {
        self.http
            .post(format!("{}{path}", self.url))
            .header("X-Appwrite-Project", &self.project)
            .header("Content-Type", "application/json")
    }
    fn jwt(&self, create: bool) -> Result<Option<String>, String> {
        let mut cached = self.session.lock().unwrap_or_else(|e| e.into_inner());
        if cached.is_none() && self.file.exists() {
            let mut bytes = Vec::new();
            std::fs::File::open(&self.file)
                .map_err(|_| "Cannot read anonymous identity")?
                .take(65537)
                .read_to_end(&mut bytes)
                .map_err(|_| "Cannot read anonymous identity")?;
            if bytes.len() > 65536 {
                return Err("Invalid anonymous identity".into());
            }
            *cached = Some(
                serde_json::from_slice(&bytes)
                    .map_err(|_| "Invalid anonymous identity; retained without replacement")?,
            );
        }
        if cached.is_none() && !create {
            return Ok(None);
        }
        if cached.is_none() {
            let reply = self
                .request("/account/sessions/anonymous")
                .body("{}")
                .send()
                .map_err(|_| "Anonymous session unavailable")?;
            let name = format!("a_session_{}=", self.project);
            let cookie = reply
                .headers()
                .get_all(reqwest::header::SET_COOKIE)
                .iter()
                .filter_map(|h| h.to_str().ok())
                .map(|h| h.split(';').next().unwrap_or_default())
                .find(|h| h.starts_with(&name))
                .map(str::to_owned);
            response(reply)?;
            let session = Session {
                cookie: cookie.ok_or("Appwrite did not return a native session cookie")?,
            };
            self.save(&session)?;
            *cached = Some(session);
        }
        let session = cached.as_ref().ok_or("Anonymous identity unavailable")?;
        if !session
            .cookie
            .starts_with(&format!("a_session_{}=", self.project))
            || session.cookie.len() > 16384
            || session.cookie.contains(['\r', '\n'])
        {
            return Err("Invalid anonymous identity".into());
        }
        let jwt = response(
            self.request("/account/jwts")
                .header(reqwest::header::COOKIE, &session.cookie)
                .body("{}")
                .send()
                .map_err(|_| "Cannot authenticate anonymous identity")?,
        )?;
        Ok(Some(
            jwt["jwt"].as_str().ok_or("Invalid anonymous JWT")?.into(),
        ))
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

    fn call(&self, body: Value, jwt: Option<&str>) -> Result<Value, String> {
        let mut headers = json!({});
        if let Some(jwt) = jwt {
            headers["x-freergb-jwt"] = json!(jwt);
        }
        let body = json!({"body":body.to_string(),"async":false,"headers":headers});
        let execution = response(
            self.request("/functions/community-api/executions")
                .body(body.to_string())
                .send()
                .map_err(|_| "Community service unavailable")?,
        )?;
        let status = execution["responseStatusCode"].as_u64().unwrap_or(0);
        if !(200..300).contains(&status) {
            return Err(format!(
                "Community action failed (HTTP {status}). Please retry."
            ));
        }
        serde_json::from_str(
            execution["responseBody"]
                .as_str()
                .ok_or("Missing community response")?,
        )
        .map_err(|_| "Invalid community response".into())
    }
    pub fn fetch(&self, effects: &[String]) -> Result<Ratings, String> {
        self.flush_downloads()?;
        let mut ratings = Ratings {
            configured: true,
            updated_at: Some(now()),
            ..Default::default()
        };
        for chunk in effects.chunks(64) {
            let data = self.call(json!({"action":"summary","effects":chunk}), None)?;
            let entries: BTreeMap<String, Summary> =
                serde_json::from_value(data["by_effect"].clone())
                    .map_err(|_| "Invalid rating summary")?;
            if entries.iter().any(|(id, s)| {
                !safe_id(id) || !s.average.is_finite() || !(0.0..=5.0).contains(&s.average)
            }) {
                return Err("Invalid rating summary".into());
            }
            ratings.by_effect.extend(entries);
        }
        if let Some(jwt) = self.jwt(false)? {
            for chunk in effects.chunks(64) {
                let data = self.call(json!({"action":"mine","effects":chunk}), Some(&jwt))?;
                let mine: BTreeMap<String, u8> = serde_json::from_value(data["mine"].clone())
                    .map_err(|_| "Invalid own ratings")?;
                if mine
                    .iter()
                    .any(|(id, s)| !safe_id(id) || !(1..=5).contains(s))
                {
                    return Err("Invalid own ratings".into());
                }
                ratings.mine.extend(mine);
            }
        }
        Ok(ratings)
    }
    pub fn rate(&self, id: &str, stars: Option<u8>) -> Result<(), String> {
        if !safe_id(id) || stars.is_some_and(|s| !(1..=5).contains(&s)) {
            return Err("Choose one to five stars".into());
        }
        let Some(jwt) = self.jwt(stars.is_some())? else {
            return Ok(());
        };
        self.call(
            json!({"action":"rate","effect_id":id,"stars":stars}),
            Some(&jwt),
        )
        .map(|_| ())
    }
    pub fn download(&self, id: &str, version: &str) -> Result<(), String> {
        let _guard = self.outbox.lock().unwrap_or_else(|e| e.into_inner());
        if !safe_id(id) || !safe_id(version) {
            return Err("Invalid download receipt".into());
        }
        use sha2::{Digest, Sha256};
        let directory = self.file.with_extension("downloads");
        std::fs::create_dir_all(&directory).map_err(|_| "Cannot queue download count")?;
        let name = hex::encode(Sha256::digest(format!("{id}/{version}")));
        let path = directory.join(format!("{name}.json"));
        std::fs::write(path, json!({"effect_id":id,"version":version}).to_string())
            .map_err(|_| "Cannot queue download count")?;
        self.flush_downloads_inner()
    }
    /// Deletes download receipts that were queued but not yet sent.
    pub fn discard_queued_downloads(&self) {
        let _guard = self.outbox.lock().unwrap_or_else(|e| e.into_inner());
        let _ = std::fs::remove_dir_all(self.file.with_extension("downloads"));
    }
    fn flush_downloads(&self) -> Result<(), String> {
        let _guard = self.outbox.lock().unwrap_or_else(|e| e.into_inner());
        self.flush_downloads_inner()
    }
    fn flush_downloads_inner(&self) -> Result<(), String> {
        let directory = self.file.with_extension("downloads");
        if !directory.exists() {
            return Ok(());
        }
        let receipts: Vec<_> = std::fs::read_dir(&directory)
            .map_err(|_| "Cannot read queued downloads")?
            .take(128)
            .collect();
        if receipts.is_empty() {
            return Ok(());
        }
        let jwt = self.jwt(true)?.ok_or("Anonymous identity unavailable")?;
        for receipt in receipts {
            let path = receipt.map_err(|_| "Cannot read queued download")?.path();
            let mut bytes = Vec::new();
            std::fs::File::open(&path)
                .map_err(|_| "Cannot read queued download")?
                .take(1025)
                .read_to_end(&mut bytes)
                .map_err(|_| "Cannot read queued download")?;
            let data: Value =
                serde_json::from_slice(&bytes).map_err(|_| "Invalid queued download")?;
            self.call(json!({"action":"download","effect_id":data["effect_id"],"version":data["version"],"success":true}),Some(&jwt))?;
            match std::fs::remove_file(path) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(_) => return Err("Cannot clear queued download".into()),
            }
        }
        Ok(())
    }
    pub fn submit(&self, kind: &str, payload: &Value, request_id: &str) -> Result<Value, String> {
        if !matches!(kind, "feedback" | "hardware" | "error")
            || !safe_id(request_id)
            || !payload.is_object()
            || payload.to_string().len() > 30000
        {
            return Err("Invalid report or report too large".into());
        }
        let jwt = self.jwt(true)?.ok_or("Anonymous identity unavailable")?;
        self.call(json!({"action":"submit","kind":kind,"payload":payload,"request_id":request_id,"reviewed":true}),Some(&jwt))
    }
    pub fn inbox(&self, action: &str, params: &Value) -> Result<Value, String> {
        if !matches!(action, "inbox" | "thread" | "reply") {
            return Err("Invalid inbox action".into());
        }
        let Some(jwt) = self.jwt(false)? else {
            return Ok(json!({"items":[],"total":0,"next":null}));
        };
        let mut body = params.clone();
        if !body.is_object() {
            body = json!({});
        }
        body["action"] = json!(action);
        self.call(body, Some(&jwt))
    }
}
