use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use netool::protocol::{Command, Response};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::Instant;
use std::net::SocketAddr;
use axum::{
    extract::{Path, State},
    routing::{get, post},
    http::StatusCode,
    Json, Router,
};
use tower_http::services::ServeDir;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Server address (IP:Port)
    #[arg(short, long, default_value = "127.0.0.1:8080")]
    target: String,

    #[command(subcommand)]
    mode: Mode,
}

#[derive(Subcommand, Debug)]
enum Mode {
    Ping,
    Speed {
        #[arg(short, long, default_value_t = 10)]
        duration: u64,
    },
    Ports {
        #[arg(short, long)]
        range: String,
    },
    Web {
        #[arg(short, long, default_value = "127.0.0.1:3000")]
        listen: String,
    },
}

#[derive(Debug, Serialize)]
struct PingResult {
    duration_ms: u128,
}

#[derive(Debug, Serialize)]
struct SpeedTestResult {
    total_bytes: u64,
    duration_secs: f64,
    mbps: f64,
}

#[derive(Debug, Serialize)]
struct PortTestResult {
    total_ports: usize,
    open_ports: Vec<u16>,
    success_count: usize,
    fail_count: usize,
}

#[derive(Clone)]
struct AppState {
    target: String,
}

#[derive(Deserialize)]
struct SpeedTestRequest {
    duration: u64,
}

#[derive(Deserialize)]
struct PortTestRequest {
    range: String,
}

async fn run_web_server(listen: &str, target: &str) -> Result<()> {
    let state = Arc::new(AppState {
        target: target.to_string(),
    });

    let app = Router::new()
        .route("/api/ping", get(ping_handler))
        .route("/api/speed", post(speed_test_handler))
        .route("/api/ports", post(port_test_handler))
        .nest_service("/", ServeDir::new("static"))
        .with_state(state);

    println!("Web server listening on {}", listen);
    let listener = tokio::net::TcpListener::bind(listen).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn ping_handler(State(state): State<Arc<AppState>>) -> Result<Json<PingResult>, (StatusCode, String)> {
    let stream = TcpStream::connect(&state.target).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let res = run_ping(stream).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(res))
}

async fn speed_test_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SpeedTestRequest>,
) -> Result<Json<SpeedTestResult>, (StatusCode, String)> {
    let stream = TcpStream::connect(&state.target).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let res = run_speed_test(stream, &state.target, payload.duration)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(res))
}

async fn port_test_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<PortTestRequest>,
) -> Result<Json<PortTestResult>, (StatusCode, String)> {
    let stream = TcpStream::connect(&state.target).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let res = run_port_test(stream, &state.target, payload.range)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(res))
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    println!("Connecting to control server at {}", args.target);

    let mut stream = TcpStream::connect(&args.target).await.context("Failed to connect to control server")?;

    match args.mode {
        Mode::Ping => {
            let res = run_ping(stream).await?;
            println!("Pong! RTT: {}ms", res.duration_ms);
        }
        Mode::Speed { duration } => {
            let res = run_speed_test(stream, &args.target, duration).await?;
            println!("Speed test finished.");
            println!("Total received: {} bytes", res.total_bytes);
            println!("Duration: {:.2}s", res.duration_secs);
            println!("Speed: {:.2} Mbps", res.mbps);
        }
        Mode::Ports { range } => {
            let res = run_port_test(stream, &args.target, range).await?;
            println!("Test complete. Success: {}, Failed: {}", res.success_count, res.fail_count);
        }
        Mode::Web { listen } => {
            run_web_server(&listen, &args.target).await?;
        }
    }

    Ok(())
}

async fn send_command(stream: &mut TcpStream, command: Command) -> Result<Response> {
    let cmd_bytes = serde_json::to_vec(&command)?;
    let cmd_len = (cmd_bytes.len() as u32).to_be_bytes();
    stream.write_all(&cmd_len).await?;
    stream.write_all(&cmd_bytes).await?;

    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf);

    let mut body = vec![0; len as usize];
    stream.read_exact(&mut body).await?;

    let response: Response = serde_json::from_slice(&body)?;
    Ok(response)
}

async fn run_ping(mut stream: TcpStream) -> Result<PingResult> {
    let start = Instant::now();
    let response = send_command(&mut stream, Command::Ping).await?;
    let duration = start.elapsed();

    match response {
        Response::Pong => Ok(PingResult {
            duration_ms: duration.as_millis(),
        }),
        _ => Err(anyhow::anyhow!("Unexpected response: {:?}", response)),
    }
}

async fn run_speed_test(mut stream: TcpStream, target_addr: &str, duration: u64) -> Result<SpeedTestResult> {
    println!("Requesting speed test for {} seconds...", duration);
    let response = send_command(&mut stream, Command::StartSpeedTest { duration_secs: duration }).await?;

    let port = match response {
        Response::SpeedTestReady { port } => port,
        _ => return Err(anyhow::anyhow!("Unexpected response: {:?}", response)),
    };

    let host = target_addr.rsplit_once(':').map(|(h, _)| h).unwrap_or("127.0.0.1");
    let data_addr = format!("{}:{}", host, port);
    println!("Connecting to data stream at {}...", data_addr);

    let mut data_stream = TcpStream::connect(data_addr).await?;
    
    let start = Instant::now();
    let mut total_bytes = 0;
    let mut buf = [0u8; 64 * 1024];

    while start.elapsed().as_secs() < duration {
        match data_stream.read(&mut buf).await {
            Ok(0) => break, // EOF
            Ok(n) => total_bytes += n,
            Err(e) => {
                eprintln!("Error reading data: {}", e);
                break;
            }
        }
    }

    let duration_secs = start.elapsed().as_secs_f64();
    let mbps = (total_bytes as f64 * 8.0) / (1_000_000.0 * duration_secs);

    Ok(SpeedTestResult {
        total_bytes: total_bytes as u64,
        duration_secs,
        mbps,
    })
}

async fn run_port_test(mut stream: TcpStream, target_addr: &str, range: String) -> Result<PortTestResult> {
    let ports = parse_ports(&range)?;
    println!("Requesting to open {} ports...", ports.len());

    // Request to open ports
    // Note: If too many ports, we might need to chunk this. 
    // For now, send all.
    let response = send_command(&mut stream, Command::OpenPorts { ports: ports.clone() }).await?;
    
    let opened_ports = match response {
        Response::PortsOpened { ports } => ports,
        _ => return Err(anyhow::anyhow!("Unexpected response: {:?}", response)),
    };

    println!("Server opened {} ports. Testing connectivity...", opened_ports.len());
    
    let host = target_addr.rsplit_once(':').map(|(h, _)| h).unwrap_or("127.0.0.1");
    let mut success_count = 0;
    let mut fail_count = 0;
    let total_ports = opened_ports.len();
    let mut last_report = Instant::now();
    let mut verified_ports = Vec::new();

    for (i, port) in opened_ports.iter().enumerate() {
        if last_report.elapsed().as_secs() >= 5 {
            println!("Progress: {}/{} ports checked ({:.1}%)", i, total_ports, (i as f64 / total_ports as f64) * 100.0);
            last_report = Instant::now();
        }

        let addr = format!("{}:{}", host, port);
        match TcpStream::connect(&addr).await {
            Ok(mut stream) => {
                // Verify handshake
                let mut buf = [0u8; 2];
                match stream.read_exact(&mut buf).await {
                    Ok(_) if &buf == b"OK" => {
                        // println!("Port {} is OPEN (Verified)", port);
                        success_count += 1;
                        verified_ports.push(*port);
                    }
                    _ => {
                        fail_count += 1;
                    }
                }
            }
            Err(_) => {
                // println!("Port {} is CLOSED/UNREACHABLE", port);
                fail_count += 1;
            }
        }
    }

    // Clean up
    println!("Closing ports...");
    send_command(&mut stream, Command::ClosePorts { ports: opened_ports }).await?;

    Ok(PortTestResult {
        total_ports,
        open_ports: verified_ports,
        success_count,
        fail_count,
    })
}

fn parse_ports(range: &str) -> Result<Vec<u16>> {
    let mut ports = Vec::new();
    for part in range.split(',') {
        if part.contains('-') {
            let parts: Vec<&str> = part.split('-').collect();
            if parts.len() == 2 {
                let start: u16 = parts[0].parse()?;
                let end: u16 = parts[1].parse()?;
                for p in start..=end {
                    ports.push(p);
                }
            }
        } else {
            let p: u16 = part.parse()?;
            ports.push(p);
        }
    }
    Ok(ports)
}
