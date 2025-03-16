use crate::{codec::{payload_bytes, read_buffer_until_payload}, config::DspClientConfig, logger::NS_CONN, protocol::*};

use anyhow::{Context, Result};
use log::info;
use tokio::{io::{AsyncWriteExt, BufReader}, net::{tcp::{OwnedReadHalf, OwnedWriteHalf}, TcpStream}};
use std::collections::VecDeque;

pub struct DspClient {
    reader: BufReader<OwnedReadHalf>,
    writer: OwnedWriteHalf,
}

impl DspClient {
    pub async fn spawn(config: &DspClientConfig) -> Result<DspClient> {
        let address_for_connection = config.server_address.clone();
        info!(target: NS_CONN, "Connecting to {}...", address_for_connection);
        let stream: TcpStream = TcpStream::connect(address_for_connection).await.with_context(|| format!("Failed to connect to DSP server at '{}'", &config.server_address))?;
        info!(target: NS_CONN, "Connected!");
        let (reader_raw, writer) = stream.into_split();
        let reader = BufReader::new(reader_raw);
        
        Ok(DspClient {
            reader,
            writer,
        })
    }

    pub async fn read_next_payload(&mut self) -> Result<DspPayload> {
        read_buffer_until_payload(&mut self.reader).await
    }

    pub async fn send_payload(&mut self, payload: DspPayload) -> Result<()> {
        let mut bytes = payload_bytes(payload);
        bytes.push(0u8);
        let mut deq = VecDeque::from(bytes);
        self.writer.write_all_buf(&mut deq).await.with_context(|| format!("Failed to send payload to socket"))
    }
}
