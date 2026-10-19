//! Reference device plugin. Stdout is exclusively the versioned wire protocol.
//! Optional --log PATH records requests; --fault MODE exercises host isolation.
use freergb_core::{LedLayout, Rgb};
use freergb_plugins::{MAX_MESSAGE, PROTOCOL_VERSION};
use serde_json::{json, Value};
use std::io::{self, BufRead, Read, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let option = |name: &str| {
        args.windows(2)
            .find(|a| a[0] == name)
            .map(|a| a[1].as_str())
    };
    let fault = option("--fault").unwrap_or("");
    if fault == "no-read" {
        std::thread::sleep(std::time::Duration::from_secs(60));
    }
    let mut log = option("--log").map(std::fs::File::create).transpose()?;
    let mut input = io::stdin().lock();
    let mut output = io::stdout().lock();
    let mut count = None;
    loop {
        let mut bytes = Vec::new();
        (&mut input)
            .take((MAX_MESSAGE + 1) as u64)
            .read_until(b'\n', &mut bytes)?;
        if bytes.is_empty() {
            return Ok(());
        }
        if bytes.len() > MAX_MESSAGE || bytes.last() != Some(&b'\n') {
            return Err("invalid message size".into());
        }
        let request: Value = serde_json::from_slice(&bytes)?;
        if let Some(log) = &mut log {
            log.write_all(&bytes)?;
            log.flush()?;
        }
        let method = request["method"].as_str().ok_or("missing method")?;
        if fault == "hang" || (fault == "hang-shutdown" && method == "shutdown") {
            std::thread::sleep(std::time::Duration::from_secs(60));
        }
        if fault == "crash" || (fault == "crash-frame" && method == "frame") {
            std::process::exit(23);
        }
        if fault == "oversized" {
            output.write_all(&vec![b'x'; MAX_MESSAGE + 1])?;
            output.flush()?;
            continue;
        }
        if fault == "malformed" {
            writeln!(output, "not json")?;
            output.flush()?;
            continue;
        }
        let params = &request["params"];
        let result = if request["api_version"] != PROTOCOL_VERSION {
            Err("unsupported version")
        } else {
            match method {
                "initialize" => {
                    let layout: LedLayout = serde_json::from_value(params["layout"].clone())?;
                    layout.validate()?;
                    count = Some(layout.len());
                    Ok(())
                }
                "frame" => {
                    let colors: Vec<Rgb> = serde_json::from_value(params["colors"].clone())?;
                    if Some(colors.len()) == count {
                        Ok(())
                    } else {
                        Err("frame size mismatch")
                    }
                }
                "shutdown" => Ok(()),
                _ => Err("unknown method"),
            }
        };
        let id = if fault == "wrong-id" {
            json!(0)
        } else {
            request["id"].clone()
        };
        let version = if fault == "wrong-version" {
            999
        } else {
            PROTOCOL_VERSION
        };
        let reply = match result {
            Ok(()) => json!({"api_version": version, "id": id, "result": true}),
            Err(error) => json!({"api_version": version, "id": id, "error": error}),
        };
        writeln!(output, "{reply}")?;
        output.flush()?;
        if method == "shutdown" {
            return Ok(());
        }
    }
}
