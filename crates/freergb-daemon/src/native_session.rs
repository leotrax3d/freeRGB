//! Rust peer for the private native-worker protocol. No listener or activation.
//! The protected supervisor must authenticate and bind the transport to its exact
//! worker before calling this module. This module provides no OS authentication.
use freergb_control::telemetry_guard::NATIVE_TELEMETRY_MAX_AGE_MS;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::time::timeout;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    Active,
    Closed,
    Emergency,
    RecoveryFailed,
}

/// Native callback and explicit return evidence; mechanical RPM proof is separate.
#[derive(Debug)]
pub struct Receipt {
    pub state: SessionState,
    pub reason: Option<String>,
    pub duty_percent: u8,
    pub owned: bool,
    pub firmware_return_verified: bool,
    pub journal_cleared: bool,
    /// Support tier reported by the worker; `None` for legacy verified workers.
    pub tier: Option<String>,
    /// Owned firmware callback completed (lower tiers have no register proof).
    pub firmware_returned: bool,
    /// Non-fatal worker condition (unrestricted fan stop).
    pub warning: Option<String>,
}

impl Receipt {
    /// A lower-tier worker returned the channel to its captured firmware mode
    /// after a fault and cleared its journal: safe, but not a normal close.
    pub fn fault_returned(&self) -> bool {
        self.tier.is_some()
            && self.state == SessionState::Emergency
            && !self.owned
            && self.firmware_returned
            && self.journal_cleared
    }
}

#[derive(Clone, Copy, Deserialize)]
struct NativeTelemetry {
    sequence: u64,
    temperature_c: f32,
    rpm: f32,
    age_ms: u64,
    #[serde(default)]
    sources: Option<NativeSourceTimes>,
}

#[derive(Clone, Copy, Deserialize)]
struct NativeSourceTimes {
    temperature_sequence: u64,
    rpm_sequence: u64,
    temperature_age_ms: u64,
    rpm_age_ms: u64,
}

#[derive(Clone, Copy)]
pub struct NativeObservation {
    pub temperature_c: f32,
    pub rpm: f32,
    pub acquired: Instant,
    pub temperature_acquired: Instant,
    pub rpm_acquired: Instant,
}

fn observation(
    raw: Option<NativeTelemetry>,
    began: Instant,
    previous: Option<(NativeTelemetry, NativeObservation)>,
) -> Result<Option<(NativeTelemetry, NativeObservation)>, String> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    if !(-20.0..=150.0).contains(&raw.temperature_c)
        || !(0.0..=100_000.0).contains(&raw.rpm)
        || raw.age_ms > NATIVE_TELEMETRY_MAX_AGE_MS
    {
        return Err("Invalid trusted native telemetry".into());
    }
    if let Some((old, observed)) = previous {
        if raw.sequence < old.sequence
            || (raw.sequence == old.sequence
                && (raw.temperature_c != old.temperature_c || raw.rpm != old.rpm))
        {
            return Err("Native sample identity changed or reversed".into());
        }
        if raw.sources.is_some() != old.sources.is_some() {
            return Err("Native telemetry source format changed".into());
        }
        if raw.sequence == old.sequence && raw.sources.is_none() {
            return Ok(Some((raw, observed)));
        }
    }
    // Request start is conservative: transport latency never makes data newer.
    let (temperature_acquired, rpm_acquired) = if let Some(sources) = raw.sources {
        if raw.age_ms != sources.temperature_age_ms.max(sources.rpm_age_ms)
            || raw.sequence != sources.rpm_sequence
        {
            return Err("Inconsistent native source ages or identity".into());
        }
        let previous =
            previous.and_then(|(old, observed)| old.sources.map(|times| (times, old, observed)));
        (
            source_acquisition(
                sources.temperature_sequence,
                sources.temperature_age_ms,
                raw.temperature_c,
                began,
                previous.map(|(times, old, observed)| {
                    (
                        times.temperature_sequence,
                        old.temperature_c,
                        observed.temperature_acquired,
                    )
                }),
            )?,
            source_acquisition(
                sources.rpm_sequence,
                sources.rpm_age_ms,
                raw.rpm,
                began,
                previous.map(|(times, old, observed)| {
                    (times.rpm_sequence, old.rpm, observed.rpm_acquired)
                }),
            )?,
        )
    } else {
        let acquired = began
            .checked_sub(Duration::from_millis(raw.age_ms))
            .ok_or("Native sample time overflow")?;
        (acquired, acquired)
    };
    Ok(Some((
        raw,
        NativeObservation {
            temperature_c: raw.temperature_c,
            rpm: raw.rpm,
            acquired: temperature_acquired.min(rpm_acquired),
            temperature_acquired,
            rpm_acquired,
        },
    )))
}

fn source_acquisition(
    sequence: u64,
    age_ms: u64,
    value: f32,
    began: Instant,
    previous: Option<(u64, f32, Instant)>,
) -> Result<Instant, String> {
    if age_ms > NATIVE_TELEMETRY_MAX_AGE_MS {
        return Err("Invalid trusted native source age".into());
    }
    if let Some((old_sequence, old_value, acquired)) = previous {
        if sequence < old_sequence || (sequence == old_sequence && value != old_value) {
            return Err("Native source identity changed or reversed".into());
        }
        if sequence == old_sequence {
            return Ok(acquired);
        }
    }
    began
        .checked_sub(Duration::from_millis(age_ms))
        .ok_or_else(|| "Native source time overflow".into())
}

// Intentionally no Debug/Serialize: ticket/session secrets must not enter logs.
#[derive(Deserialize)]
struct Reply {
    version: u32,
    session: String,
    ticket: String,
    sequence: i32,
    state: SessionState,
    reason: Option<String>,
    duty_percent: u8,
    ticket_ms: u32,
    lease_ms: u32,
    owned: bool,
    physical_write_verified: bool,
    #[serde(default)]
    firmware_return_verified: bool,
    #[serde(default)]
    journal_cleared: bool,
    #[serde(default)]
    telemetry: Option<NativeTelemetry>,
    #[serde(default)]
    tier: Option<String>,
    #[serde(default)]
    firmware_returned: bool,
    #[serde(default)]
    warning: Option<String>,
}

impl Reply {
    fn floor(&self) -> u8 {
        if self.tier.as_deref() == Some("unrestricted") {
            0
        } else {
            40
        }
    }
}

#[derive(Serialize)]
struct Request<'a> {
    session: &'a str,
    ticket: &'a str,
    sequence: i32,
    action: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    duty_percent: Option<u8>,
}

pub struct NativeSessionClient<T> {
    transport: Option<T>,
    reply: Reply,
    received: Instant,
    observation: Option<(NativeTelemetry, NativeObservation)>,
}

fn token(value: &str) -> bool {
    value.len() == 32
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

async fn read<T: AsyncRead + Unpin>(transport: &mut T) -> Result<Reply, String> {
    let bytes = timeout(Duration::from_secs(2), async {
        let mut bytes = Vec::new();
        for _ in 0..2048 {
            let byte = transport
                .read_u8()
                .await
                .map_err(|_| "Native reply disconnected")?;
            if byte == b'\n' {
                return Ok(bytes);
            }
            bytes.push(byte);
        }
        Err("Native reply exceeds bound")
    })
    .await
    .map_err(|_| "Native reply timed out")??;
    let reply: Reply =
        serde_json::from_slice(&bytes).map_err(|_| "Invalid native reply encoding")?;
    let tiered = reply.tier.is_some();
    // A lower tier may return to firmware itself after a fault (owned callback,
    // journal cleared); legacy verified workers only report this when closed.
    let fault_return =
        tiered && reply.state == SessionState::Emergency && !reply.owned && reply.firmware_returned;
    if reply.version != 1
        || !token(&reply.session)
        || !token(&reply.ticket)
        || reply.sequence < 0
        || !matches!(
            reply.tier.as_deref(),
            None | Some("verified" | "lhm_supported" | "unrestricted")
        )
        || reply
            .warning
            .as_ref()
            .is_some_and(|value| !tiered || value.len() > 64)
        || (reply.firmware_returned
            && (!tiered || (reply.state != SessionState::Closed && !fault_return)))
        || !(reply.floor()..=100).contains(&reply.duty_percent)
        || reply.ticket_ms != 250
        || reply.lease_ms != 1000
        || reply.physical_write_verified
        || reply
            .reason
            .as_ref()
            .is_some_and(|value| value.len() > 512 || value.chars().count() > 128)
        || (reply.state == SessionState::Active && (!reply.owned || reply.reason.is_some()))
        || (reply.state == SessionState::Closed && (reply.owned || reply.duty_percent != 100))
        || (reply.firmware_return_verified && reply.state != SessionState::Closed)
        || (reply.journal_cleared && reply.state != SessionState::Closed && !fault_return)
    {
        return Err("Invalid native reply semantics".into());
    }
    Ok(reply)
}

impl<T: AsyncRead + AsyncWrite + Unpin> NativeSessionClient<T> {
    /// Caller supplies an already authenticated, worker-lifetime-bound transport.
    /// Cancellation/drop closes the transport; no reconnect or command replay.
    pub async fn from_authenticated_transport(mut transport: T) -> Result<Self, String> {
        let began = Instant::now();
        let reply = read(&mut transport).await?;
        if reply.state != SessionState::Active || reply.sequence != 0 || reply.duty_percent != 100 {
            return Err("Unsafe initial native session".into());
        }
        let observation = observation(reply.telemetry, began, None)?;
        Ok(Self {
            transport: Some(transport),
            reply,
            received: Instant::now(),
            observation,
        })
    }

    pub fn observation(&self) -> Option<NativeObservation> {
        self.observation.map(|(_, sample)| sample)
    }

    /// Tier the protected worker admitted; `None` for legacy verified workers.
    pub fn tier(&self) -> Option<&str> {
        self.reply.tier.as_deref()
    }

    pub async fn set_duty(&mut self, duty: u8) -> Result<Receipt, String> {
        if !(self.reply.floor()..=100).contains(&duty) {
            return Err("Native duty is outside the admitted tier's range".into());
        }
        self.send(Some(duty)).await
    }

    /// Closed alone means callback completion. Explicit return/journal evidence
    /// must also be checked before authorizing maintenance.
    pub async fn close(&mut self) -> Result<Receipt, String> {
        self.send(None).await
    }

    async fn send(&mut self, duty: Option<u8>) -> Result<Receipt, String> {
        let began = Instant::now();
        // Move out before the first await: cancellation drops this transport too,
        // and leaves the client unusable instead of permitting a duplicate send.
        let mut transport = self.transport.take().ok_or("Native session is terminal")?;
        if self.reply.state != SessionState::Active
            || self.received.elapsed() > Duration::from_millis(200)
        {
            return Err("Native ticket expired or session terminal".into());
        }
        let next = self
            .reply
            .sequence
            .checked_add(1)
            .ok_or("Native counter exhausted")?;
        let mut command = serde_json::to_vec(&Request {
            session: &self.reply.session,
            ticket: &self.reply.ticket,
            sequence: next,
            action: if duty.is_some() { "set" } else { "close" },
            duty_percent: duty,
        })
        .map_err(|_| "Native command encoding failed")?;
        command.push(b'\n');
        timeout(Duration::from_secs(2), async {
            transport.write_all(&command).await?;
            transport.flush().await
        })
        .await
        .map_err(|_| "Native command timed out")?
        .map_err(|_| "Native command disconnected")?;
        let reply = read(&mut transport).await?;
        let next_observation = observation(reply.telemetry, began, self.observation)?;
        if self.observation.is_some()
            && next_observation.is_none()
            && reply.state == SessionState::Active
        {
            return Err("Trusted native sampler became unavailable".into());
        }
        let regular = matches!(reply.state, SessionState::Active | SessionState::Closed);
        if reply.session != self.reply.session
            || reply.tier != self.reply.tier
            || (regular && reply.sequence != next)
            || (!regular && !(self.reply.sequence..=next).contains(&reply.sequence))
            || (reply.state == SessionState::Active
                && (reply.ticket == self.reply.ticket || duty != Some(reply.duty_percent)))
            || (reply.state == SessionState::Closed && duty.is_some())
        {
            return Err("Native reply does not match this operation".into());
        }
        let receipt = Receipt {
            state: reply.state,
            reason: reply.reason.clone(),
            duty_percent: reply.duty_percent,
            owned: reply.owned,
            firmware_return_verified: reply.firmware_return_verified,
            journal_cleared: reply.journal_cleared,
            tier: reply.tier.clone(),
            firmware_returned: reply.firmware_returned,
            warning: reply.warning.clone(),
        };
        self.received = Instant::now();
        if reply.state == SessionState::Active {
            self.transport = Some(transport);
        }
        self.reply = reply;
        self.observation = next_observation;
        Ok(receipt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};
    use tokio::io::{AsyncBufReadExt, BufReader};
    #[test]
    fn fresh_rpm_cannot_renew_a_shared_temperature_or_change_its_identity() {
        let began = Instant::now();
        let raw = NativeTelemetry {
            sequence: 10,
            temperature_c: 45.0,
            rpm: 900.0,
            age_ms: 1000,
            sources: Some(NativeSourceTimes {
                temperature_sequence: 1,
                rpm_sequence: 10,
                temperature_age_ms: 1000,
                rpm_age_ms: 0,
            }),
        };
        let first = observation(Some(raw), began, None).unwrap().unwrap();
        let times = NativeSourceTimes {
            rpm_sequence: 11,
            temperature_age_ms: 1100,
            ..raw.sources.unwrap()
        };
        let newer = NativeTelemetry {
            sequence: 11,
            rpm: 901.0,
            age_ms: 1100,
            sources: Some(times),
            ..raw
        };
        let next = observation(Some(newer), began + Duration::from_millis(100), Some(first))
            .unwrap()
            .unwrap();
        assert_eq!(first.1.temperature_acquired, next.1.temperature_acquired);
        assert!(next.1.rpm_acquired > first.1.rpm_acquired);
        assert_eq!(next.1.acquired, first.1.acquired);
        assert!(observation(
            Some(NativeTelemetry {
                temperature_c: 46.0,
                ..newer
            }),
            began,
            Some(first)
        )
        .is_err());
        assert!(observation(
            Some(NativeTelemetry {
                sources: None,
                ..newer
            }),
            began,
            Some(first)
        )
        .is_err());
        assert!(observation(
            Some(NativeTelemetry { age_ms: 0, ..newer }),
            began,
            Some(first)
        )
        .is_err());
    }
    #[test]
    fn native_cached_sample_identity_cannot_refresh_acquisition_time() {
        let began = Instant::now();
        let raw = NativeTelemetry {
            sequence: 10,
            temperature_c: 40.0,
            rpm: 900.0,
            age_ms: 50,
            sources: None,
        };
        let first = observation(Some(raw), began, None).unwrap().unwrap();
        let repeated = observation(
            Some(NativeTelemetry { age_ms: 0, ..raw }),
            began + Duration::from_secs(1),
            Some(first),
        )
        .unwrap()
        .unwrap();
        assert_eq!(first.1.acquired, repeated.1.acquired);
        let delayed = observation(
            Some(NativeTelemetry {
                age_ms: 7000,
                ..raw
            }),
            began + Duration::from_secs(7),
            None,
        )
        .unwrap()
        .unwrap();
        assert_eq!(delayed.1.acquired, began);
        assert!(observation(
            Some(NativeTelemetry {
                age_ms: 7001,
                ..raw
            }),
            began + Duration::from_secs(8),
            None,
        )
        .is_err());
        assert!(observation(
            Some(NativeTelemetry { rpm: 0.0, ..raw }),
            began,
            Some(first)
        )
        .is_err());
        assert!(observation(
            Some(NativeTelemetry { sequence: 9, ..raw }),
            began,
            Some(first)
        )
        .is_err());
    }
    fn reply(sequence: i32, ticket: char, state: &str, duty: u8, owned: bool) -> Value {
        json!({"version":1,"session":"a".repeat(32),"ticket":ticket.to_string().repeat(32),"sequence":sequence,
            "state":state,"reason":null,"duty_percent":duty,"ticket_ms":250,"lease_ms":1000,"owned":owned,"physical_write_verified":false})
    }
    async fn write(stream: &mut tokio::io::DuplexStream, value: Value) {
        stream
            .write_all(format!("{value}\n").as_bytes())
            .await
            .unwrap();
    }
    #[tokio::test]
    async fn real_stream_set_and_close_preserve_callback_only_evidence() {
        let (client, mut worker) = tokio::io::duplex(4096);
        let task = tokio::spawn(async move {
            write(&mut worker, reply(0, 'b', "active", 100, true)).await;
            let mut worker = BufReader::new(worker);
            let mut line = String::new();
            worker.read_line(&mut line).await.unwrap();
            let command: Value = serde_json::from_str(&line).unwrap();
            assert_eq!(command["sequence"], 1);
            assert_eq!(command["duty_percent"], 40);
            write(worker.get_mut(), reply(1, 'c', "active", 40, true)).await;
            line.clear();
            worker.read_line(&mut line).await.unwrap();
            let command: Value = serde_json::from_str(&line).unwrap();
            assert_eq!(command["action"], "close");
            assert!(command.get("duty_percent").is_none());
            write(worker.get_mut(), reply(2, 'd', "closed", 100, false)).await;
        });
        let mut client = NativeSessionClient::from_authenticated_transport(client)
            .await
            .unwrap();
        assert_eq!(client.set_duty(40).await.unwrap().duty_percent, 40);
        let receipt = client.close().await.unwrap();
        assert_eq!(receipt.state, SessionState::Closed);
        assert!(!receipt.owned);
        assert!(client.set_duty(40).await.is_err());
        task.await.unwrap();
    }
    #[tokio::test]
    async fn corrupt_reply_closes_transport_without_replay() {
        for field in [
            "session",
            "sequence",
            "ticket",
            "duty_percent",
            "physical_write_verified",
            "firmware_return_verified",
            "journal_cleared",
        ] {
            let (client, mut worker) = tokio::io::duplex(4096);
            let task = tokio::spawn(async move {
                write(&mut worker, reply(0, 'b', "active", 100, true)).await;
                let mut worker = BufReader::new(worker);
                let mut line = String::new();
                worker.read_line(&mut line).await.unwrap();
                let mut bad = reply(1, 'c', "active", 40, true);
                bad[field] = match field {
                    "session" => json!("e".repeat(32)),
                    "sequence" => json!(2),
                    "ticket" => json!("b".repeat(32)),
                    "duty_percent" => json!(50),
                    _ => json!(true),
                };
                write(worker.get_mut(), bad).await;
                line.clear();
                assert_eq!(worker.read_line(&mut line).await.unwrap(), 0);
            });
            let mut client = NativeSessionClient::from_authenticated_transport(client)
                .await
                .unwrap();
            assert!(client.set_duty(40).await.is_err());
            assert!(client.set_duty(40).await.is_err());
            task.await.unwrap();
        }
    }
    fn tiered(
        sequence: i32,
        ticket: char,
        state: &str,
        duty: u8,
        owned: bool,
        tier: &str,
    ) -> Value {
        let mut value = reply(sequence, ticket, state, duty, owned);
        value["tier"] = json!(tier);
        value["firmware_returned"] = json!(false);
        value["warning"] = Value::Null;
        value
    }
    #[tokio::test]
    async fn unrestricted_tier_permits_fan_stop_and_reports_fault_return() {
        let (client, mut worker) = tokio::io::duplex(4096);
        let task = tokio::spawn(async move {
            write(
                &mut worker,
                tiered(0, 'b', "active", 100, true, "unrestricted"),
            )
            .await;
            let mut worker = BufReader::new(worker);
            let mut line = String::new();
            worker.read_line(&mut line).await.unwrap();
            let command: Value = serde_json::from_str(&line).unwrap();
            assert_eq!(command["duty_percent"], 0);
            let mut stopped = tiered(1, 'c', "active", 0, true, "unrestricted");
            stopped["warning"] = json!("fan_stalled");
            write(worker.get_mut(), stopped).await;
            line.clear();
            worker.read_line(&mut line).await.unwrap();
            let mut returned = tiered(1, 'c', "emergency", 100, false, "unrestricted");
            returned["reason"] = json!("lease_expired");
            returned["firmware_returned"] = json!(true);
            returned["journal_cleared"] = json!(true);
            write(worker.get_mut(), returned).await;
        });
        let mut client = NativeSessionClient::from_authenticated_transport(client)
            .await
            .unwrap();
        assert_eq!(client.tier(), Some("unrestricted"));
        let receipt = client.set_duty(0).await.unwrap();
        assert_eq!(
            (receipt.duty_percent, receipt.warning.as_deref()),
            (0, Some("fan_stalled"))
        );
        let receipt = client.set_duty(50).await.unwrap();
        assert!(receipt.fault_returned());
        task.await.unwrap();
    }
    #[tokio::test]
    async fn lower_tiers_cannot_claim_unrestricted_ranges_or_switch_tier() {
        for (initial, bad) in [
            (
                tiered(0, 'b', "active", 100, true, "lhm_supported"),
                tiered(1, 'c', "active", 0, true, "lhm_supported"),
            ),
            (
                tiered(0, 'b', "active", 100, true, "lhm_supported"),
                tiered(1, 'c', "active", 40, true, "unrestricted"),
            ),
            (reply(0, 'b', "active", 100, true), {
                let mut v = reply(1, 'c', "emergency", 100, false);
                v["journal_cleared"] = json!(true);
                v["firmware_returned"] = json!(true);
                v
            }),
            (tiered(0, 'b', "active", 100, true, "lhm_supported"), {
                let mut v = tiered(1, 'c', "active", 40, true, "lhm_supported");
                v["journal_cleared"] = json!(true);
                v
            }),
        ] {
            let (client, mut worker) = tokio::io::duplex(4096);
            let task = tokio::spawn(async move {
                write(&mut worker, initial).await;
                let mut worker = BufReader::new(worker);
                let mut line = String::new();
                worker.read_line(&mut line).await.unwrap();
                write(worker.get_mut(), bad).await;
            });
            let mut client = NativeSessionClient::from_authenticated_transport(client)
                .await
                .unwrap();
            assert!(client.set_duty(40).await.is_err());
            task.await.unwrap();
        }
        let (client, mut worker) = tokio::io::duplex(4096);
        write(&mut worker, reply(0, 'b', "active", 100, true)).await;
        let mut client = NativeSessionClient::from_authenticated_transport(client)
            .await
            .unwrap();
        assert!(client.set_duty(39).await.is_err());
    }
    #[tokio::test]
    async fn oversized_unterminated_reply_is_bounded() {
        let (client, mut worker) = tokio::io::duplex(4096);
        worker.write_all(&[b'x'; 2048]).await.unwrap();
        assert!(NativeSessionClient::from_authenticated_transport(client)
            .await
            .is_err());
    }
    #[tokio::test]
    async fn cancelled_send_drops_transport_and_cannot_be_replayed() {
        let (client, mut worker) = tokio::io::duplex(4096);
        let task = tokio::spawn(async move {
            write(&mut worker, reply(0, 'b', "active", 100, true)).await;
            let mut worker = BufReader::new(worker);
            let mut line = String::new();
            worker.read_line(&mut line).await.unwrap();
            assert!(!line.is_empty());
            // Simulate a worker whose native callback never produces a receipt.
            line.clear();
            assert_eq!(
                timeout(Duration::from_secs(2), worker.read_line(&mut line))
                    .await
                    .unwrap()
                    .unwrap(),
                0
            );
        });
        let mut client = NativeSessionClient::from_authenticated_transport(client)
            .await
            .unwrap();
        assert!(timeout(Duration::from_millis(50), client.set_duty(40))
            .await
            .is_err());
        assert!(client.set_duty(40).await.is_err());
        task.await.unwrap();
    }
}
