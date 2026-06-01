use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream, ToSocketAddrs};
use std::time::Duration;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum TcpError {
    #[error("connect failed: {0}")]
    Connect(String),
    #[error("write failed: {0}")]
    Write(String),
    #[error("read failed: {0}")]
    Read(String),
    #[error("not connected")]
    NotConnected,
}

pub type TcpResult<T> = Result<T, TcpError>;

#[derive(Debug, Clone)]
pub struct TcpClientConfig {
    pub host: String,
    pub port: u16,
    pub timeout_ms: u64,
}

impl TcpClientConfig {
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
            timeout_ms: 3000,
        }
    }

    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

pub struct TcpClient {
    config: TcpClientConfig,
    stream: Option<TcpStream>,
}

impl TcpClient {
    pub fn new(config: TcpClientConfig) -> Self {
        Self {
            config,
            stream: None,
        }
    }

    pub fn connect(&mut self) -> TcpResult<()> {
        let timeout = Duration::from_millis(self.config.timeout_ms);
        let address = self.config.address();
        let socket_addr = address
            .to_socket_addrs()
            .map_err(|e| TcpError::Connect(e.to_string()))?
            .next()
            .ok_or_else(|| TcpError::Connect(format!("invalid address: {}", address)))?;

        let stream = TcpStream::connect_timeout(&socket_addr, timeout)
            .map_err(|e| TcpError::Connect(e.to_string()))?;
        stream
            .set_read_timeout(Some(timeout))
            .map_err(|e| TcpError::Connect(e.to_string()))?;
        stream
            .set_write_timeout(Some(timeout))
            .map_err(|e| TcpError::Connect(e.to_string()))?;
        stream
            .set_nodelay(true)
            .map_err(|e| TcpError::Connect(e.to_string()))?;

        self.stream = Some(stream);
        Ok(())
    }

    pub fn write(&mut self, data: &[u8]) -> TcpResult<usize> {
        let stream = self.stream.as_mut().ok_or(TcpError::NotConnected)?;
        stream
            .write_all(data)
            .map_err(|e| TcpError::Write(e.to_string()))?;
        Ok(data.len())
    }

    pub fn read(&mut self, max_bytes: usize) -> TcpResult<Vec<u8>> {
        let stream = self.stream.as_mut().ok_or(TcpError::NotConnected)?;
        let mut buf = vec![0u8; max_bytes];
        let n = stream
            .read(&mut buf)
            .map_err(|e| TcpError::Read(e.to_string()))?;
        buf.truncate(n);
        Ok(buf)
    }

    pub fn query(&mut self, request: &[u8], max_bytes: usize) -> TcpResult<Vec<u8>> {
        self.write(request)?;
        self.read(max_bytes)
    }

    pub fn disconnect(&mut self) {
        if let Some(stream) = self.stream.take() {
            let _ = stream.shutdown(Shutdown::Both);
        }
    }
}

impl Drop for TcpClient {
    fn drop(&mut self) {
        self.disconnect();
    }
}
