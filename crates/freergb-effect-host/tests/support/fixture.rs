//! Synthetic peer only. Never opens hardware or starts the daemon.
use std::io::{BufRead, Write};
fn main() {
    let mode = std::env::args().nth(1).unwrap_or_default();
    if mode == "no-read" {
        std::thread::sleep(std::time::Duration::from_secs(60));
        return;
    }
    for line in std::io::stdin().lock().lines() {
        let request: serde_json::Value = serde_json::from_str(&line.unwrap()).unwrap();
        match mode.as_str() {
            "crash" => std::process::exit(13),
            "hang" => std::thread::sleep(std::time::Duration::from_secs(60)),
            "malformed" => println!("invalid"),
            "oversized" => println!("{}", "x".repeat(768 * 1024 + 1)),
            "wrong-id" => println!(
                "{}",
                serde_json::json!({"api":1,"id":999,"result":{"Ok":""}})
            ),
            "wrong-version" => println!(
                "{}",
                serde_json::json!({"api":999,"id":request["id"],"result":{"Ok":""}})
            ),
            _ => std::process::exit(2),
        }
        std::io::stdout().flush().unwrap();
    }
}
