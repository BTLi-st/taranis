//! 保存配置

use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

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

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Conf {
    #[serde(rename = "price")]
    pub price: PriceConf,
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
        let deserialized_conf: Conf = toml::from_str(&toml_str).expect("Failed to deserialize from TOML");
        assert_eq!(conf.price.path, deserialized_conf.price.path);
    }
}