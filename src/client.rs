use crate::{codec::{payload_bytes, read_buffer_until_payload}, config::DspClientConfig, logger::NS_CONN, protocol::*};

use anyhow::{Context, Result};
use log::info;
use tokio::{io::{AsyncWriteExt, BufReader}, net::{tcp::{OwnedReadHalf, OwnedWriteHalf}, TcpStream}};
use std::collections::VecDeque;

pub struct DspReader {
    underlying: BufReader<OwnedReadHalf>
}

impl DspReader {
    pub async fn read(&mut self) -> Result<DspPayload> {
        read_buffer_until_payload(&mut self.underlying).await
    }
}

pub struct DspWriter {
    underlying: OwnedWriteHalf
}

impl DspWriter {
    pub async fn write(&mut self, payload: DspPayload) -> Result<()> {
        let mut bytes = payload_bytes(payload);
        bytes.push(0u8);
        let mut deq = VecDeque::from(bytes);
        self.underlying.write_all_buf(&mut deq).await.with_context(|| format!("Failed to send payload to socket"))
    }
}

pub struct DspClient {
    pub reader: DspReader,
    pub writer: DspWriter,
}

impl DspClient {
    pub async fn spawn(config: &DspClientConfig) -> Result<DspClient> {
        let address_for_connection = config.server_address.clone();
        info!(target: NS_CONN, "Connecting to {}...", address_for_connection);
        let stream: TcpStream = TcpStream::connect(address_for_connection).await.with_context(|| format!("Failed to connect to DSP server at '{}'", &config.server_address))?;
        info!(target: NS_CONN, "Connected!");
        let (reader_raw, writer_raw) = stream.into_split();
        let reader = DspReader { underlying: BufReader::new(reader_raw) };
        let writer = DspWriter { underlying: writer_raw };
        
        Ok(DspClient {
            reader,
            writer,
        })
    }
}
