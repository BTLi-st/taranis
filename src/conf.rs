//! 保存配置

use std::sync::LazyLock;

use chrono::DateTime;
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
    #[serde(default = "disallow_break")]
    /// 是否允许中断充电
    pub allow_break: bool,
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

fn disallow_break() -> bool {
    false // 默认不允许中断充电
}

impl Default for ChargeConf {
    fn default() -> Self {
        ChargeConf {
            charge_type: default_charge_type(), // 默认充电类型为快速充电
            power: default_power(),             // 默认功率为30kW
            size: default_size(),               // 默认队列大小为2
            allow_break: false,                 // 默认允许中断充电
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TimeConf {
    #[serde(default = "default_update_interval")]
    /// 更新间隔，单位为毫秒
    pub update_interval: u64,
    #[serde(default = "default_tz")]
    /// 时区
    pub tz: Tz,
    #[serde(default = "default_speed")]
    /// 加速倍数
    pub speed: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// 开始时间
    pub start_time: Option<DateTime<chrono::Utc>>,
}

fn default_update_interval() -> u64 {
    5000 // 默认更新间隔为5000毫秒（5秒）
}

fn default_tz() -> Tz {
    "Asia/Shanghai".parse().unwrap() // 默认时区为上海
}

fn default_speed() -> u64 {
    1 // 默认加速倍数为1
}

impl Default for TimeConf {
    fn default() -> Self {
        TimeConf {
            update_interval: default_update_interval(),
            tz: default_tz(),
            speed: default_speed(),
            start_time: None, // 默认没有开始时间（开始时间为系统当前时间）
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
    #[serde(rename = "time", default = "TimeConf::default")]
    /// 时间配置
    pub time: TimeConf,
}

/// 静态配置实例，使用 LazyLock 确保在第一次访问时加载配置文件
pub static CONF: LazyLock<Conf> = LazyLock::new(|| {
    let path = "config.toml";
    let conf = if let Ok(content) = std::fs::read_to_string(path) {
        tracing::info!("加载配置文件: {}", path);
        toml::from_str(&content).unwrap_or_else(|_| {
            tracing::warn!("配置文件解析失败，使用默认配置");
            Conf::default()
        })
    } else {
        tracing::debug!("配置文件不存在: {}，使用默认配置", path);
        Conf::default()
    };
    tracing::debug!("配置文件内容: {:?}", conf);
    tracing::info!("充电桩类型: {:?}", conf.charge.charge_type);
    tracing::info!("充电功率: {} kW", conf.charge.power);
    if conf.time.start_time.is_some() {
        tracing::info!("配置文件中指定了开始时间: {:?}", conf.time.start_time);
    } else {
        tracing::info!("配置文件中未指定开始时间，使用当前系统时间");
    }
    if conf.time.update_interval < 100 {
        tracing::warn!(
            "时间更新间隔过短: {} 毫秒，可能会导致性能问题",
            conf.time.update_interval
        );
    }
    if conf.time.speed == 0 {
        tracing::error!("时间加速比为 0，可能会导致严重的运行问题");
    } else if conf.time.speed > 1 {
        tracing::warn!(
            "时间加速比为 {}，过高的加速可能会导致不准确的时间计算",
            conf.time.speed
        );
    }
    conf
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
