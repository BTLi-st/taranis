use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    #[serde(rename = "register")]
    Register,
    #[serde(rename = "unregister")]
    Update,
    #[serde(rename = "update")]
    Complete,
    #[serde(rename = "fault")]
    Fault,
    #[serde(rename = "new")]
    New,
    #[serde(rename = "cancel")]
    Cancel,
    #[serde(rename = "close")]
    Close,
    #[serde(rename = "open")]
    Open,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MSG {
    #[serde(rename = "type")]
    pub type_: MessageType, // 消息类型
    pub data: String, // 消息数据
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_serialization() {
        let message = MSG {
            type_: MessageType::Register,
            data: "Test data".to_string(),
        };
        let serialized = serde_json::to_string(&message).unwrap();
        assert!(serialized.contains("\"type\":\"register\""));
        assert!(serialized.contains("\"data\":\"Test data\""));
    }

    #[test]
    fn test_message_deserialization() {
        let json = r#"{"type":"update","data":"Update data"}"#;
        let message: MSG = serde_json::from_str(json).unwrap();
        assert_eq!(message.type_, MessageType::Update);
        assert_eq!(message.data, "Update data");
    }
}