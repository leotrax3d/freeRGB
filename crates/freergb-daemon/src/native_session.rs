//! Rust peer for the private native-worker protocol. No listener or activation.
//! The protected supervisor must authenticate and bind the transport to its exact
//! worker before calling this module. This module provides no OS authentication.
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

/// Callback-level evidence only. Never authorizes firmware-return verification.
#[derive(Debug)]
pub struct Receipt {
    pub state: SessionState,
    pub reason: Option<String>,
    pub duty_percent: u8,
    pub owned: bool,
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
    if reply.version != 1
        || !token(&reply.session)
        || !token(&reply.ticket)
        || reply.sequence < 0
        || !(40..=100).contains(&reply.duty_percent)
        || reply.ticket_ms != 250
        || reply.lease_ms != 1000
        || reply.physical_write_verified
        || reply
            .reason
            .as_ref()
            .is_some_and(|value| value.len() > 512 || value.chars().count() > 128)
        || (reply.state == SessionState::Active && (!reply.owned || reply.reason.is_some()))
        || (reply.state == SessionState::Closed && (reply.owned || reply.duty_percent != 100))
    {
        return Err("Invalid native reply semantics".into());
    }
    Ok(reply)
}

impl<T: AsyncRead + AsyncWrite + Unpin> NativeSessionClient<T> {
    /// Caller supplies an already authenticated, worker-lifetime-bound transport.
    /// Cancellation/drop closes the transport; no reconnect or command replay.
    pub async fn from_authenticated_transport(mut transport: T) -> Result<Self, String> {
        let reply = read(&mut transport).await?;
        if reply.state != SessionState::Active || reply.sequence != 0 || reply.duty_percent != 100 {
            return Err("Unsafe initial native session".into());
        }
        Ok(Self {
            transport: Some(transport),
            reply,
            received: Instant::now(),
        })
    }

    pub async fn set_duty(&mut self, duty: u8) -> Result<Receipt, String> {
        if !(40..=100).contains(&duty) {
            return Err("Native duty must be 40..100 percent".into());
        }
        self.send(Some(duty)).await
    }

    /// Closed means native callback returned, NOT verified firmware restoration.
    pub async fn close(&mut self) -> Result<Receipt, String> {
        self.send(None).await
    }

    async fn send(&mut self, duty: Option<u8>) -> Result<Receipt, String> {
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
        let regular = matches!(reply.state, SessionState::Active | SessionState::Closed);
        if reply.session != self.reply.session
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
        };
        self.received = Instant::now();
        if reply.state == SessionState::Active {
            self.transport = Some(transport);
        }
        self.reply = reply;
        Ok(receipt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};
    use tokio::io::{AsyncBufReadExt, BufReader};
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
