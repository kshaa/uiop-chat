use crate::protocol::*;
use std::{mem, str::FromStr};
use anyhow::{anyhow, Context, Result};
use log::warn;
use nom::{bytes::complete::{tag, take_while_m_n}, combinator::{flat_map, map, map_res, opt}, IResult, Parser};
use tokio::io::AsyncBufReadExt;

fn username(input: &str) -> IResult<&str, &str> {
    take_while_m_n(1, 32, |c: char| c.is_alphanumeric() || c == '_').parse(input)
}

fn message_type(input: &str) -> IResult<&str, MessageType> {
    map_res(
        take_while_m_n(1, 20, |c: char| c.is_alphabetic() && c.is_uppercase()),
        MessageType::from_str
    ).parse(input)
}

fn join_message(input: &str) -> IResult<&str, JoinMessage> {
    Ok((input, JoinMessage {}))
}

fn dsp_join_message(input: &str) -> IResult<&str, DspMessage> {
    map(join_message, DspMessage::JoinMessage).parse(input)
}

fn quit_message(input: &str) -> IResult<&str, QuitMessage> {
    Ok((input, QuitMessage {}))
}

fn dsp_quit_message(input: &str) -> IResult<&str, DspMessage> {
    map(quit_message, DspMessage::QuitMessage).parse(input)
}

fn message_message(input: &str) -> IResult<&str, MessageMessage> {
    Ok(("", MessageMessage { text: input.to_string() }))
}

fn dsp_message_message(input: &str) -> IResult<&str, DspMessage> {
    map(message_message, DspMessage::MessageMessage).parse(input)
}

fn challenge_message(input: &str) -> IResult<&str, ChallengeMessage> {
    map(
        (
            take_while_m_n(1, 99, |c: char| c.is_numeric()),
            tag(" "),
            take_while_m_n(0, 64, |c: char| c.is_alphanumeric()),
        ),
        |(n, _, phrase): (&str, &str, &str)| {
            ChallengeMessage { n: n.len() as u64, phrase: phrase.to_string() }
        }
    ).parse(input)
}

fn dsp_challange_message(input: &str) -> IResult<&str, DspMessage> {
    map(challenge_message, DspMessage::ChallengeMessage).parse(input)
}

fn rescinded_message(input: &str) -> IResult<&str, RescindedMessage> {
    Ok((input, RescindedMessage { }))
}

fn dsp_rescinded_message(input: &str) -> IResult<&str, DspMessage> {
    map(rescinded_message, DspMessage::RescindedMessage).parse(input)
}

fn response_message(input: &str) -> IResult<&str, ResponseMessage> {
    Ok(("", ResponseMessage { phrase: input.to_string() }))
}

fn dsp_response_message(input: &str) -> IResult<&str, DspMessage> {
    map(response_message, DspMessage::ResponseMessage).parse(input)
}

fn error_message(input: &str) -> IResult<&str, ErrorMessage> {
    Ok(("", ErrorMessage { text: input.to_string() }))
}

fn dsp_error_message(input: &str) -> IResult<&str, DspMessage> {
    map(error_message, DspMessage::ErrorMessage).parse(input)
}

fn message_of_type<'a>(message_type: MessageType) -> impl Parser<&'a str, Output = DspMessage, Error = nom::error::Error<&'a str>> {
    match message_type {
        MessageType::JOIN => dsp_join_message,
        MessageType::QUIT => dsp_quit_message,
        MessageType::MESSAGE => dsp_message_message,
        MessageType::CHALLENGE => dsp_challange_message,
        MessageType::RESCINDED => dsp_rescinded_message,
        MessageType::RESPONSE => dsp_response_message,
        MessageType::ERROR => dsp_error_message,
    }
}

fn message_with_type(input: &str) -> IResult<&str, DspMessage> {
    flat_map((message_type, opt(tag(" "))), |(mtype, _)| message_of_type(mtype)).parse(input)
}

fn payload(input: &str) -> IResult<&str, DspPayload> {
    map(
        (
            username,
            tag(" "),
            message_with_type,
        ),
        |(username, _, message)| {
            DspPayload { username: username.to_string(), message }
        }
    ).parse(input)
}

fn parse_payload(input: String) -> Result<DspPayload> {
    payload(&input).map(|(_, payload)| payload).map_err(|e| anyhow!(e.to_string())).with_context(|| format!("Failed to parse DSP payload"))
}

fn stringify_payload(input: DspPayload) -> String {
    let username = input.username;
    let message = match input.message {
        DspMessage::JoinMessage(_) => format!("JOIN"),
        DspMessage::QuitMessage(_) => format!("QUIT"),
        DspMessage::MessageMessage(m) => format!("MESSAGE {}", m.text),
        DspMessage::ChallengeMessage(m) => format!("CHALLENGE {} {}", Vec::<char>::with_capacity(m.n as usize).into_iter().map(|_| '0').collect::<String>(), m.phrase),
        DspMessage::RescindedMessage(_) => format!("RESCINDED"),
        DspMessage::ResponseMessage(m) => format!("RESPONSE {}", m.phrase),
        DspMessage::ErrorMessage(m) => format!("ERROR {}", m.text),
    };
    format!("{} {}", username, message)
}

pub fn payload_bytes(input: DspPayload) -> Vec<u8> {
    stringify_payload(input).into_bytes()
}

pub async fn read_buffer_until_payload<R: AsyncBufReadExt + Unpin>(buf_reader: &mut R) -> Result<DspPayload> {
    loop {
        // Define reader buffers
        let mut byte_payload = vec![];
        
        // Read until \0
        let payload_read = buf_reader.read_until(b'\0', &mut byte_payload);
        let read_count = payload_read.await.with_context(|| format!("Failed while reading next message bytes"))?;

        // Check if connection closed
        if read_count == 0 {
            return Err(anyhow!("Reached EOF while reading next message bytes, assuming connection closed"))
        }

        // Drop null-terminator if present
        if byte_payload.last() == Some(&0u8) {
            byte_payload.remove(byte_payload.len() - 1);
        }

        // Parse bytes into UTF-8
        let text_payload = match String::from_utf8(mem::take(&mut byte_payload)).with_context(|| format!("Received message is not a valid UTF-8 byte stream, ignoring")) {
            Err(err) => {
                warn!("{}", err);
                continue
            },
            Ok(text_payload) => text_payload,
        };

        // Parse text into message payload
        let message_payload = parse_payload(text_payload).with_context(|| format!("Failed to parse UTF-8 byte message as a DSP message payload"))?;

        // Return valid message payload
        return Ok(message_payload);
    }
}

#[cfg(test)]
mod tests {
    use nom::AsBytes;

    use super::*;

    fn serde_check(text: &str, data: DspPayload) {
        let de = parse_payload(text.to_string()).map_err(|e| e.to_string());
        assert_eq!(de, Ok(data));
        let ser = stringify_payload(de.unwrap());
        assert_eq!(ser, text.to_string());
    }

    #[test]
    fn check_message_encoding() {
        serde_check(
            "testuser JOIN", 
            DspPayload { 
                username: String::from("testuser"), 
                message: DspMessage::JoinMessage(JoinMessage {})
            }
        );
        serde_check(
            "testuser MESSAGE This is a great message !@#%^&* 123 :)", 
            DspPayload { 
                username: String::from("testuser"), 
                message: DspMessage::MessageMessage(MessageMessage { text: String::from("This is a great message !@#%^&* 123 :)")})
            }
        );
    }

    #[tokio::test]
    async fn check_payload_buffer_read() {
        let text = format!("testuser MESSAGE test");
        let text_buf = text.as_bytes();
        let terminator_buf = &[0u8];
        let concat_buf = [text_buf, terminator_buf].concat();
        let mut buf = concat_buf.as_bytes();
        let result= read_buffer_until_payload(&mut buf).await.map_err(|e| e.to_string());
        assert_eq!(
            result, 
            Ok(DspPayload { 
                username: String::from("testuser"), 
                message: DspMessage::MessageMessage(MessageMessage { text: String::from("test")})
            })
        );
    }
}
