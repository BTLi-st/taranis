//! 保存配置

use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

use chrono_tz::Tz;

#[derive(Debug, Serialize, Deserialize)]
pub struct PriceConf {
    #[serde(default = "price_conf_path")]
    pub path: String,
}

fn price_conf_path() -> String {
    "prices.json".to_string()
}

impl Default for PriceConf {
    fn default() -> Self {
        PriceConf {
            path: "prices.json".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChargeType {
    #[serde(rename = "F")]
    Fast,
    #[serde(rename = "T")]
    Slow,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChargeConf {
    pub charge_type: ChargeType,
    pub power: f64, // 充电功率
    #[serde(default = "default_size")]
    pub size: u32, // 队列大小
    #[serde(default = "default_tz")]
    pub tz: Tz, // 时区
    #[serde(default = "disallow_break")]
    pub allow_break: bool, // 是否允许中断充电
    #[serde(default = "default_update_interval")]
    pub update_interval: u64, // 更新间隔，单位为秒
}

fn default_size() -> u32 {
    2 // 默认队列大小为2
}

fn default_tz() -> Tz {
    "Asia/Shanghai".parse().unwrap() // 默认时区为上海
}

fn disallow_break() -> bool {
    false // 默认不允许中断充电
}

fn default_update_interval() -> u64 {
    5 // 默认更新间隔为5秒
}

impl Default for ChargeConf {
    fn default() -> Self {
        ChargeConf {
            charge_type: ChargeType::Fast,
            power: 30.0,          // 默认功率为7.2kW
            size: default_size(), // 默认队列大小为2
            tz: default_tz(),     // 默认时区为上海
            allow_break: false,   // 默认允许中断充电
            update_interval: default_update_interval(), // 默认更新间隔为5秒
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebSocketConf {
    #[serde(default = "default_websocket_url")]
    pub url: String,
}

fn default_websocket_url() -> String {
    "ws://localhost:8080/ws".to_string() // 默认WebSocket URL
}

impl Default for WebSocketConf {
    fn default() -> Self {
        WebSocketConf {
            url: default_websocket_url(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Conf {
    #[serde(rename = "price")]
    pub price: PriceConf,
    #[serde(rename = "charge")]
    pub charge: ChargeConf,
    #[serde(rename = "websocket")]
    pub websocket: WebSocketConf,
}

pub static CONF: LazyLock<Conf> = LazyLock::new(|| {
    let path = "config.toml";
    if let Ok(content) = std::fs::read_to_string(path) {
        tracing::info!("加载配置文件: {}", path);
        toml::from_str(&content).unwrap_or_else(|_| Conf::default())
    } else {
        tracing::debug!("配置文件不存在: {}，使用默认配置", path);
        Conf::default()
    }
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conf_serialization() {
        let conf = Conf::default();
        let toml_str = toml::to_string(&conf).expect("Failed to serialize to TOML");
        println!("Serialized TOML:\n{}", toml_str);
        let deserialized_conf: Conf =
            toml::from_str(&toml_str).expect("Failed to deserialize from TOML");
        assert_eq!(conf.price.path, deserialized_conf.price.path);
    }
}
