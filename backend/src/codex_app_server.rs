use crate::codex_limits::{
    packet_from_rate_limits_result_value, CodexLimitsPacket, STATUS_CODEX_ERROR, STATUS_STALE,
};
use serde_json::{json, Value};
use std::error::Error;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::RwLock;
use tokio::time::{Duration, Instant};

type DynError = Box<dyn Error + Send + Sync>;

const POLL_INTERVAL: Duration = Duration::from_secs(60);
const STALE_AFTER: Duration = Duration::from_secs(90);

pub struct CodexAppServer {
    child: Child,
    stdin: ChildStdin,
    stdout: Lines<BufReader<ChildStdout>>,
    next_id: u64,
}

#[derive(Clone)]
pub struct CodexLimitsCache {
    inner: Arc<RwLock<CachedPacket>>,
}

struct CachedPacket {
    packet: CodexLimitsPacket,
    updated_at: Option<Instant>,
}

impl CodexLimitsCache {
    pub fn start_polling() -> Self {
        let cache = Self {
            inner: Arc::new(RwLock::new(CachedPacket {
                packet: CodexLimitsPacket::error(),
                updated_at: None,
            })),
        };

        let worker_cache = cache.clone();
        tokio::spawn(async move {
            loop {
                if let Err(err) = poll_until_process_exits(worker_cache.clone()).await {
                    eprintln!("Codex app-server polling failed: {err}");
                    worker_cache.mark_error().await;
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        });

        cache
    }

    pub async fn current_packet(&self) -> CodexLimitsPacket {
        let cached = self.inner.read().await;

        match cached.updated_at {
            Some(updated_at) if updated_at.elapsed() > STALE_AFTER => {
                cached.packet.with_status(STATUS_STALE)
            }
            Some(_) => cached.packet.clone(),
            None => cached.packet.with_status(STATUS_CODEX_ERROR),
        }
    }

    async fn store(&self, packet: CodexLimitsPacket) {
        let mut cached = self.inner.write().await;
        cached.packet = packet;
        cached.updated_at = Some(Instant::now());
    }

    async fn mark_error(&self) {
        let mut cached = self.inner.write().await;
        if cached.updated_at.is_none() {
            cached.packet = CodexLimitsPacket::error();
        }
    }
}

impl CodexAppServer {
    pub async fn start() -> Result<Self, DynError> {
        let mut child = Command::new("codex")
            .arg("app-server")
            .arg("--listen")
            .arg("stdio://")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;

        let stdin = child
            .stdin
            .take()
            .ok_or("failed to open codex app-server stdin")?;
        let stdout = child
            .stdout
            .take()
            .ok_or("failed to open codex app-server stdout")?;

        let mut client = Self {
            child,
            stdin,
            stdout: BufReader::new(stdout).lines(),
            next_id: 1,
        };
        client.initialize().await?;
        Ok(client)
    }

    pub async fn read_rate_limits_packet(&mut self) -> Result<CodexLimitsPacket, DynError> {
        let result = self.request("account/rateLimits/read").await?;
        packet_from_rate_limits_result_value(result)
    }

    async fn initialize(&mut self) -> Result<(), DynError> {
        self.send(json!({
            "method": "initialize",
            "id": 0,
            "params": {
                "clientInfo": {
                    "name": "codex_flipper_monitor",
                    "title": "Codex Flipper Monitor",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }
        }))
        .await?;

        self.read_response_result(0).await?;

        self.send(json!({
            "method": "initialized",
            "params": {}
        }))
        .await?;

        Ok(())
    }

    async fn request(&mut self, method: &str) -> Result<Value, DynError> {
        let id = self.next_id;
        self.next_id += 1;
        self.send(json!({
            "method": method,
            "id": id
        }))
        .await?;

        self.read_response_result(id).await
    }

    async fn send(&mut self, value: Value) -> Result<(), DynError> {
        let mut bytes = serde_json::to_vec(&value)?;
        bytes.push(b'\n');
        self.stdin.write_all(&bytes).await?;
        self.stdin.flush().await?;
        Ok(())
    }

    async fn read_response_result(&mut self, id: u64) -> Result<Value, DynError> {
        while let Some(line) = self.stdout.next_line().await? {
            let value: Value = serde_json::from_str(&line)?;
            if value.get("id").and_then(Value::as_u64) != Some(id) {
                continue;
            }

            if let Some(error) = value.get("error") {
                return Err(format!("codex app-server error: {error}").into());
            }

            return value
                .get("result")
                .cloned()
                .ok_or_else(|| "codex app-server response missing result".into());
        }

        Err("codex app-server stdout closed".into())
    }
}

impl Drop for CodexAppServer {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
    }
}

pub async fn run_smoke_test() -> Result<(), DynError> {
    let login_status = Command::new("codex")
        .arg("login")
        .arg("status")
        .output()
        .await?;
    let stdout = String::from_utf8_lossy(&login_status.stdout);
    let stderr = String::from_utf8_lossy(&login_status.stderr);
    let login_output = format!("{stdout}{stderr}");

    if !login_status.status.success() || !login_output.contains("Logged in using ChatGPT") {
        return Err(format!(
            "codex is not logged in with ChatGPT: {}",
            login_output.trim()
        )
        .into());
    }

    let mut client = CodexAppServer::start().await?;
    let packet = client.read_rate_limits_packet().await?;

    println!("{}", login_output.trim());
    println!(
        "Codex limits: 5H {}%, 1W {}%, status {}",
        packet.five_hour_used_percent, packet.week_used_percent, packet.status
    );
    Ok(())
}

async fn poll_until_process_exits(cache: CodexLimitsCache) -> Result<(), DynError> {
    let mut client = CodexAppServer::start().await?;

    loop {
        let packet = client.read_rate_limits_packet().await?;
        cache.store(packet).await;
        tokio::time::sleep(POLL_INTERVAL).await;
    }
}
