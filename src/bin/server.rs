use anyhow::Result;
use clap::Parser;
use netool::protocol::{Command, Response};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration, Instant};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Port to listen on for control commands
    #[arg(short, long, default_value_t = 8080)]
    port: u16,
}

struct ServerState {
    // We store the JoinHandle to abort the listener task when closing ports.
    open_ports: HashMap<u16, tokio::task::JoinHandle<()>>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    println!("Starting netool-server on port {}", args.port);

    let listener = TcpListener::bind(("::", args.port)).await?;
    let state = Arc::new(Mutex::new(ServerState {
        open_ports: HashMap::new(),
    }));

    loop {
        let (socket, addr) = listener.accept().await?;
        println!("New connection from {}", addr);
        let state = state.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_connection(socket, state).await {
                eprintln!("Error handling connection: {}", e);
            }
        });
    }
}

async fn handle_connection(mut socket: TcpStream, state: Arc<Mutex<ServerState>>) -> Result<()> {
    loop {
        // Simple framing: Length (u32) + Body
        let mut len_buf = [0u8; 4];
        match socket.read_exact(&mut len_buf).await {
            Ok(_) => {},
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(()), // Client closed cleanly
            Err(e) => return Err(e.into()),
        }
        let len = u32::from_be_bytes(len_buf);
        
        let mut body = vec![0; len as usize];
        socket.read_exact(&mut body).await?;
        
        let command: Command = serde_json::from_slice(&body)?;
        println!("Received command: {:?}", command);

        let response = match command {
            Command::Ping => Response::Pong,
            Command::StartSpeedTest { duration_secs } => {
                handle_speed_test(duration_secs).await?
            }
            Command::OpenPorts { ports } => {
                handle_open_ports(ports, state.clone()).await
            }
            Command::ClosePorts { ports } => {
                handle_close_ports(ports, state.clone()).await
            }
        };

        let resp_bytes = serde_json::to_vec(&response)?;
        let resp_len = (resp_bytes.len() as u32).to_be_bytes();
        socket.write_all(&resp_len).await?;
        socket.write_all(&resp_bytes).await?;
    }
}

async fn handle_speed_test(duration_secs: u64) -> Result<Response> {
    // Bind to port 0 to let OS pick a random port
    let listener = TcpListener::bind(":::0").await?;
    let port = listener.local_addr()?.port();

    tokio::spawn(async move {
        if let Ok((mut stream, _)) = listener.accept().await {
            // Send data for duration_secs
            let start = Instant::now();
            let chunk = vec![1u8; 64 * 1024]; // 64KB chunks
            while start.elapsed().as_secs() < duration_secs {
                if stream.write_all(&chunk).await.is_err() {
                    break;
                }
            }
        }
    });

    Ok(Response::SpeedTestReady { port })
}

async fn handle_open_ports(ports: Vec<u16>, state: Arc<Mutex<ServerState>>) -> Response {
    let mut opened = Vec::new();
    let mut state_guard = state.lock().await;

    for port in ports {
        if state_guard.open_ports.contains_key(&port) {
            opened.push(port);
            continue;
        }

        match TcpListener::bind(("::", port)).await {
            Ok(listener) => {
                let handle = tokio::spawn(async move {
                    loop {
                        match listener.accept().await {
                            Ok((mut stream, _)) => {
                                // Verify connectivity by sending a small handshake
                                if let Err(e) = stream.write_all(b"OK").await {
                                    eprintln!("Failed to write to verification stream: {}", e);
                                }
                            }
                            Err(e) => {
                                eprintln!("Accept error on port {}: {}", port, e);
                                break;
                            }
                        }
                    }
                });
                state_guard.open_ports.insert(port, handle);
                opened.push(port);
            }
            Err(e) => {
                eprintln!("Failed to bind port {}: {}", port, e);
            }
        }
    }

    Response::PortsOpened { ports: opened }
}

async fn handle_close_ports(ports: Vec<u16>, state: Arc<Mutex<ServerState>>) -> Response {
    let mut closed = Vec::new();
    let mut state_guard = state.lock().await;

    for port in ports {
        if let Some(handle) = state_guard.open_ports.remove(&port) {
            handle.abort();
            closed.push(port);
        }
    }

    Response::PortsClosed { ports: closed }
}
