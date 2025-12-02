use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum Command {
    Ping,
    StartSpeedTest { duration_secs: u64 },
    OpenPorts { ports: Vec<u16> },
    ClosePorts { ports: Vec<u16> },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Response {
    Pong,
    SpeedTestReady { port: u16 },
    PortsOpened { ports: Vec<u16> },
    PortsClosed { ports: Vec<u16> },
    Error { message: String },
}
