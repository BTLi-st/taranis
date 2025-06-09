use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
/// 消息类型枚举
pub enum MessageType {
    #[serde(rename = "register")]
    /// 注册消息
    Register,
    #[serde(rename = "update")]
    /// 更新消息
    Update,
    #[serde(rename = "complete")]
    /// 完成消息
    Complete,
    #[serde(rename = "fault")]
    /// 故障消息
    Fault,
    #[serde(rename = "new")]
    /// 新消息
    New,
    #[serde(rename = "cancel")]
    /// 取消消息
    Cancel,
    #[serde(rename = "close")]
    /// 关闭消息
    Close,
    #[serde(rename = "open")]
    /// 打开消息
    Open,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
/// 消息结构体
pub struct MSG {
    #[serde(rename = "type")]
    /// 消息类型
    pub type_: MessageType, 
    /// 消息数据
    pub data: String,
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