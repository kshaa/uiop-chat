use crate::{
    codec::{payload_bytes, read_buffer_until_payload},
    config::DspClientConfig,
    logger::NS_CONN,
    protocol::{DspMessage, *},
};

use anyhow::{Context, Result};
use log::debug;
use std::collections::VecDeque;
use tokio::{
    io::{AsyncWriteExt, BufReader},
    net::{
        TcpStream,
        tcp::{OwnedReadHalf, OwnedWriteHalf},
    },
};

pub struct DspReader {
    underlying: BufReader<OwnedReadHalf>,
}

impl DspReader {
    pub async fn read(&mut self) -> Result<DspPayload> {
        read_buffer_until_payload(&mut self.underlying).await
    }
}

pub struct DspWriter {
    underlying: OwnedWriteHalf,
}

impl DspWriter {
    pub async fn write(&mut self, payload: DspPayload) -> Result<()> {
        let mut bytes = payload_bytes(payload);
        bytes.push(0u8);
        let mut deq = VecDeque::from(bytes);
        self.underlying
            .write_all_buf(&mut deq)
            .await
            .with_context(|| format!("Failed to send payload to socket"))?;
        self.underlying
            .flush()
            .await
            .with_context(|| format!("Failed to send (flush) payload to socket"))?;

        Ok(())
    }
}

pub struct DspClient {
    pub reader: DspReader,
    pub writer: DspWriter,
}

impl DspClient {
    pub async fn start(config: &DspClientConfig) -> Result<DspClient> {
        // Init connection
        let address_for_connection = config.server_address.clone();
        debug!(target: NS_CONN, "Connecting to {}...", address_for_connection);
        let stream: TcpStream = TcpStream::connect(address_for_connection)
            .await
            .with_context(|| {
                format!(
                    "Failed to connect to DSP server at '{}'",
                    &config.server_address
                )
            })?;
        debug!(target: NS_CONN, "Connected!");

        // Split connection into RW
        let (reader_raw, writer_raw) = stream.into_split();
        let reader = DspReader {
            underlying: BufReader::new(reader_raw),
        };
        let mut writer = DspWriter {
            underlying: writer_raw,
        };

        // Join the server
        writer
            .write(DspPayload {
                username: config.username.clone(),
                message: DspMessage::JoinMessage(JoinMessage {}),
            })
            .await?;

        Ok(DspClient { reader, writer })
    }
}
