//! 保存配置

use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

use chrono_tz::Tz;

#[derive(Debug, Serialize, Deserialize, Clone)]
/// 价格配置
pub struct PriceConf {
    #[serde(default = "price_conf_path")]
    /// 价格配置文件路径
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
/// 充电类型枚举
pub enum ChargeType {
    #[serde(rename = "F")]
    /// 快速充电
    Fast,
    #[serde(rename = "T")]
    /// 慢速充电
    Slow,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
/// 充电配置
pub struct ChargeConf {
    /// 充电类型
    #[serde(default = "default_charge_type")]
    pub charge_type: ChargeType,
    /// 充电功率，单位为kW
    #[serde(default = "default_power")]
    pub power: f64,
    #[serde(default = "default_size")]
    /// 队列大小
    pub size: u32,
    #[serde(default = "default_tz")]
    /// 时区
    pub tz: Tz,
    #[serde(default = "disallow_break")]
    /// 是否允许中断充电
    pub allow_break: bool,
    #[serde(default = "default_update_interval")]
    /// 更新间隔，单位为秒
    pub update_interval: u64,
}

fn default_charge_type() -> ChargeType {
    ChargeType::Fast // 默认充电类型为快速充电
}

fn default_power() -> f64 {
    30.0 // 默认功率为30kW
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
            charge_type: default_charge_type(), // 默认充电类型为快速充电
            power: default_power(), // 默认功率为30kW
            size: default_size(), // 默认队列大小为2
            tz: default_tz(),     // 默认时区为上海
            allow_break: false,   // 默认允许中断充电
            update_interval: default_update_interval(), // 默认更新间隔为5秒
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
/// WebSocket配置
pub struct WebSocketConf {
    #[serde(default = "default_websocket_url")]
    /// WebSocket URL
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

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
/// 全局配置
pub struct Conf {
    #[serde(rename = "price", default = "PriceConf::default")]
    /// 价格配置
    pub price: PriceConf,
    #[serde(rename = "charge", default = "ChargeConf::default")]
    /// 充电配置
    pub charge: ChargeConf,
    #[serde(rename = "websocket", default = "WebSocketConf::default")]
    /// WebSocket配置
    pub websocket: WebSocketConf,
}

/// 静态配置实例，使用 LazyLock 确保在第一次访问时加载配置文件
pub static CONF: LazyLock<Conf> = LazyLock::new(|| {
    let path = "config.toml";
    if let Ok(content) = std::fs::read_to_string(path) {
        tracing::info!("加载配置文件: {}", path);
        toml::from_str(&content).unwrap_or_else(|_| {
            tracing::warn!("配置文件解析失败，使用默认配置");
            Conf::default()
        })
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
