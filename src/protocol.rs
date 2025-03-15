use strum_macros::EnumString;

#[derive(Debug, PartialEq, EnumString)]
pub enum MessageType {
    JOIN,
    QUIT,
    MESSAGE,
    CHALLENGE,
    RESCINDED,
    RESPONSE,
    ERROR,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinMessage {}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuitMessage {}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageMessage { pub text: String }
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChallengeMessage { pub n: u64, pub phrase: String }
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RescindedMessage {}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResponseMessage { pub phrase: String }
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorMessage { pub text: String }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DspMessage {
    JoinMessage(JoinMessage),
    QuitMessage(QuitMessage),
    MessageMessage(MessageMessage),
    ChallengeMessage(ChallengeMessage),
    RescindedMessage(RescindedMessage),
    ResponseMessage(ResponseMessage),
    ErrorMessage(ErrorMessage),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DspPayload {
    pub username: String,
    pub message: DspMessage,
}