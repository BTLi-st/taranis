use std::sync::LazyLock;

use chrono::{NaiveDateTime, NaiveTime};
use serde::{Deserialize, Serialize};

use crate::conf::CONF;

#[derive(Serialize, Deserialize, Clone, Copy)]
struct TimePeriod {
    start: NaiveTime,
    end: NaiveTime,
    price: f64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Prices {
    periods: Vec<TimePeriod>,
    service_fee: f64,
    #[serde(default = "not_optimized", skip)]
    is_optimized: bool, // 是否经过优化
}

fn not_optimized() -> bool {
    false
}

impl Prices {
    #[allow(unused)]
    pub fn new() -> Self {
        Prices {
            periods: Vec::new(),
            service_fee: 0.0,    // 默认服务费为 0
            is_optimized: false, // 默认未优化
        }
    }

    #[allow(unused)]
    /// 添加一个时间段
    pub fn add_period(&mut self, start: NaiveTime, end: NaiveTime, price: f64) {
        let period = TimePeriod { start, end, price };
        self.periods.push(period);
    }

    /// 优化时间段，排序、合并重叠时间段、处理跨越 0 点的时间段
    /// 对于价格不一致的重叠时间段会报错
    pub fn optimize(&mut self) -> Result<&mut Self, String> {
        if self.periods.is_empty() {
            return Ok(self);
        }

        // 遍历时间段，统计跨越 0 点的时间段
        let mut cnt = 0;
        let mut new_periods = Vec::new();
        for period in &self.periods {
            if period.start > period.end && period.end != MIDNIGHT {
                cnt += 1;
                new_periods.push(TimePeriod {
                    start: period.start,
                    end: MIDNIGHT,
                    price: period.price,
                });
                new_periods.push(TimePeriod {
                    start: MIDNIGHT,
                    end: period.end,
                    price: period.price,
                });
            } else {
                new_periods.push(*period);
            }
        }
        if cnt > 1 {
            return Err(format!(
                "{} time periods cross midnight, please check your input",
                cnt
            ));
        }

        // 按照开始时间排序
        new_periods.sort_by(|a, b| a.start.cmp(&b.start));

        // 合并重叠的时间段，给空出的时间段补零，对于价格不一致重叠的时间段报错
        let mut merged_periods = Vec::new();
        let mut current_period: Option<TimePeriod> = None;

        for period in new_periods {
            if let Some(ref mut current) = current_period {
                if period.start < current.end {
                    // 重叠或相连的时间段
                    if period.price != current.price {
                        return Err(
                            "Overlapping time periods with different prices found".to_string()
                        );
                    }
                    current.end = period.end; // 扩展当前时间段的结束时间
                } else if period.start > current.end {
                    // 有空隙的时间段
                    merged_periods.push(*current); // 添加当前时间段
                    // 添加一个空的时间段
                    merged_periods.push(TimePeriod {
                        start: current.end,
                        end: period.start,
                        price: 0.0, // 空隙时间段的价格为 0
                    });
                    current_period = Some(period); // 更新当前时间段为新的时间段
                } else {
                    // 相等的时间段，直接添加
                    merged_periods.push(*current);
                    current_period = Some(period);
                }
            } else {
                if period.start != MIDNIGHT {
                    // 如果当前时间段不是从 0 点开始，则添加一个跨越 0 点的时间段
                    merged_periods.push(TimePeriod {
                        start: MIDNIGHT,
                        end: period.start,
                        price: 0.0, // 空隙时间段的价格为 0
                    });
                }
                current_period = Some(period);
            }
        }

        // 添加最后一个时间段
        if let Some(current) = current_period {
            merged_periods.push(current);
        }

        if merged_periods.last().unwrap().end != MIDNIGHT {
            // 如果最后一个时间段没有跨越 0 点，则添加一个跨越 0 点的时间段
            merged_periods.push(TimePeriod {
                start: merged_periods.last().unwrap().end,
                end: MIDNIGHT,
                price: 0.0, // 空隙时间段的价格为 0
            });
        }

        // 更新价格列表
        self.periods = merged_periods;
        self.is_optimized = true; // 标记为已优化

        Ok(self)
    }
}

static MIDNIGHT: NaiveTime = NaiveTime::from_hms_opt(0, 0, 0).unwrap();

static DEFAULT_PRICES: LazyLock<Prices> = LazyLock::new(|| {
    Prices {
        periods: vec![
            TimePeriod {
                // 谷时
                start: MIDNIGHT,
                end: NaiveTime::from_hms_opt(7, 0, 0).unwrap(),
                price: 0.4,
            },
            TimePeriod {
                // 平时
                start: NaiveTime::from_hms_opt(7, 0, 0).unwrap(),
                end: NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
                price: 0.7,
            },
            TimePeriod {
                // 峰时
                start: NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
                end: NaiveTime::from_hms_opt(15, 0, 0).unwrap(),
                price: 1.0,
            },
            TimePeriod {
                // 平时
                start: NaiveTime::from_hms_opt(15, 0, 0).unwrap(),
                end: NaiveTime::from_hms_opt(18, 0, 0).unwrap(),
                price: 0.7,
            },
            TimePeriod {
                // 峰时
                start: NaiveTime::from_hms_opt(18, 0, 0).unwrap(),
                end: NaiveTime::from_hms_opt(21, 0, 0).unwrap(),
                price: 1.0,
            },
            TimePeriod {
                // 平时
                start: NaiveTime::from_hms_opt(21, 0, 0).unwrap(),
                end: NaiveTime::from_hms_opt(23, 0, 0).unwrap(),
                price: 0.7,
            },
            TimePeriod {
                // 谷时
                start: NaiveTime::from_hms_opt(23, 0, 0).unwrap(),
                end: MIDNIGHT,
                price: 0.4,
            },
        ],
        service_fee: 0.8,    // 默认服务费为 0.8
        is_optimized: true, // 默认已优化
    }
});

impl Default for Prices {
    fn default() -> Self {
        DEFAULT_PRICES.clone()
    }
}

fn hours_to_midnight(time: NaiveTime) -> f64 {
    let midnight = NaiveTime::from_hms_opt(0, 0, 0).unwrap();
    let duration = midnight.signed_duration_since(time); // 计算从指定时间到午夜的持续时间
    24.0 + duration.num_seconds() as f64 / 3600.0 // 转换为小时
}

fn round_to_precision(value: f64, decimal_places: u32) -> f64 {
    let multiplier = 10.0_f64.powi(decimal_places as i32);
    (value * multiplier).round() / multiplier
}

impl Prices {
    /// 计算指定时间段的价格
    /// 时间段结尾不能是 0 点
    fn calc_day_price(&self, start: NaiveTime, end: NaiveTime, power: f64) -> Result<f64, String> {
        if !self.is_optimized {
            return Err("Prices have not been optimized, cannot calculate day price".to_string());
        }
        if start >= end {
            return Err("Start time must be before end time".to_string());
        }
        let mut total_price = 0.0;
        for period in &self.periods[..self.periods.len() - 1] {
            if period.start < end && period.end > start {
                // 计算重叠时间段的价格
                let overlap_start = start.max(period.start);
                let overlap_end = end.min(period.end);
                let duration = (overlap_end - overlap_start).num_seconds() as f64 / 3600.0; // 转换为小时
                total_price += duration * period.price * power;
                total_price += self.service_fee * power * duration; // 添加服务费
            }
        }
        // 特判最后一段到 0 点的时间段
        if end > self.periods.last().unwrap().start {
            let overlap_start = start.max(self.periods.last().unwrap().start);
            let duration = (end - overlap_start).num_seconds() as f64 / 3600.0; // 转换为小时
            total_price += duration * self.periods.last().unwrap().price * power;
            total_price += self.service_fee * power * duration; // 添加服务费
        }
        Ok(total_price)
    }

    fn calc_day_price_until_midnight(
        &self,
        start: NaiveTime,
        power: f64,
    ) -> Result<f64, String> {
        if !self.is_optimized {
            return Err("Prices have not been optimized, cannot calculate day price until midnight".to_string());
        }
        let mut total_price = 0.0;
        for period in &self.periods[..self.periods.len() - 1] {
            if period.end > start {
                // 计算重叠时间段的价格
                let overlap_start = start.max(period.start);
                let duration = (period.end - overlap_start).num_seconds() as f64 / 3600.0; // 转换为小时
                total_price += duration * period.price * power;
                total_price += self.service_fee * power * duration; // 添加服务费
            }
        }

        // 特判最后一段到 0 点的时间段
        let overlap_start = start.max(self.periods.last().unwrap().start);
        let duration = hours_to_midnight(overlap_start);
        total_price += duration * self.periods.last().unwrap().price * power;
        total_price += self.service_fee * power * duration; // 添加服务费

        Ok(total_price)
    }

    pub fn calc_price(
        &self,
        start: NaiveDateTime,
        end: NaiveDateTime,
        power: f64,
    ) -> Result<f64, String> {
        if !self.is_optimized {
            return Err("Prices not have been optimized, cannot calculate price".to_string());
        }
        if start >= end {
            return Err("Start time must be before end time".to_string());
        }
        let mut start_time = start.time();
        let end_time = end.time();
        let mut date = start.date();
        let mut total_price = 0.0;
        while date < end.date() {
            total_price += self.calc_day_price_until_midnight(start_time, power)?;
            date = date.succ_opt().unwrap(); // 前进到下一天
            start_time = MIDNIGHT; // 重置开始时间为午夜
        }
        // 处理最后一天的时间段
        if end_time != MIDNIGHT {
            total_price += self.calc_day_price(start_time, end_time, power)?;
        }

        Ok(round_to_precision(total_price, 2))
    }
}

static PRICESS: LazyLock<Prices> = LazyLock::new(|| {
    let path = &CONF.price.path;
    match std::fs::read_to_string(path) {
        Ok(content) => {
            let mut prices = serde_json::from_str(&content).unwrap_or_else(|e| {
                tracing::warn!("无法解析价格配置文件 {}: {}，使用默认价格表", path, e);
                let default_prices = Prices::default();
                if let Err(we) =
                    std::fs::write(path, serde_json::to_string_pretty(&default_prices).unwrap())
                {
                    tracing::error!("无法写入默认价格到 {}: {}", path, we);
                    panic!("Unable to write default prices to {}: {}", path, we);
                } else {
                    tracing::info!("默认价格已写入 {}", path);
                }
                default_prices
            });
            if let Err(e) = prices.optimize() {
                tracing::error!("价格配置文件 {} 处理失败: {}", path, e);
                panic!("Failed to optimize prices from {}: {}", path, e);
            }
            prices
        }
        Err(_) => {
            // 如果文件不存在或读取失败，返回默认价格
            tracing::warn!("价格配置文件 {} 不存在或无法读取，创建默认价格表", path);
            let default_prices = Prices::default();
            if let Err(e) =
                std::fs::write(path, serde_json::to_string_pretty(&default_prices).unwrap())
            {
                tracing::error!("无法写入默认价格到 {}: {}", path, e);
            } else {
                tracing::info!("默认价格已写入 {}", path);
            }
            default_prices
        }
    }
});

pub fn calc_price(start: NaiveDateTime, end: NaiveDateTime, power: f64) -> Result<f64, String> {
    PRICESS.calc_price(start, end, power)
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_time_period_serialization() {
        use super::*;
        let period = TimePeriod {
            start: NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            end: NaiveTime::from_hms_opt(17, 0, 0).unwrap(),
            price: 100.0,
        };

        let serialized = serde_json::to_string_pretty(&period).unwrap();
        println!("Serialized: \n{}", serialized);

        let deserialized: TimePeriod = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.start, period.start);
        assert_eq!(deserialized.end, period.end);
        assert_eq!(deserialized.price, period.price);
    }

    #[test]
    fn tests_prices_serialization() {
        use super::*;
        let prices = Prices {
            periods: vec![
                TimePeriod {
                    start: NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
                    end: NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
                    price: 50.0,
                },
                TimePeriod {
                    start: NaiveTime::from_hms_opt(13, 0, 0).unwrap(),
                    end: NaiveTime::from_hms_opt(17, 0, 0).unwrap(),
                    price: 75.0,
                },
            ],
            service_fee: 0.0,    // 默认服务费为 0
            is_optimized: false, // 默认未优化
        };
        let serialized = serde_json::to_string_pretty(&prices).unwrap();
        println!("Serialized: \n{}", serialized);
        let deserialized: Prices = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.periods.len(), 2);
        assert_eq!(deserialized.periods[0].price, 50.0);
        assert_eq!(deserialized.periods[1].price, 75.0);
    }

    #[test]
    fn test_prices_optimize() {
        use super::*;
        let mut prices = Prices {
            periods: vec![
                TimePeriod {
                    start: NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
                    end: NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
                    price: 50.0,
                },
                TimePeriod {
                    start: NaiveTime::from_hms_opt(11, 0, 0).unwrap(),
                    end: NaiveTime::from_hms_opt(15, 0, 0).unwrap(),
                    price: 50.0,
                },
                TimePeriod {
                    start: NaiveTime::from_hms_opt(16, 0, 0).unwrap(),
                    end: NaiveTime::from_hms_opt(18, 0, 0).unwrap(),
                    price: 75.0,
                },
            ],
            service_fee: 0.0,
            is_optimized: false, // 默认未优化
        }; // 默认服务费为 0
        let result = prices.optimize();
        assert!(result.is_ok());
        let optimized_prices = result.unwrap();
        println!(
            "Optimized Prices: {}",
            serde_json::to_string_pretty(optimized_prices).unwrap()
        );
    }

    #[test]
    fn test_calc_price() {
        use super::*;
        let prices = Prices::default();
        let start =
            NaiveDateTime::parse_from_str("2023-10-01 08:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let end =
            NaiveDateTime::parse_from_str("2023-10-01 20:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let power = 1.0; // 假设功率为 1.0
        let result = prices.calc_price(start, end, power).unwrap();
        println!("Calculated price: {}", result);
        assert_eq!(result, 20.1);
        let start =
            NaiveDateTime::parse_from_str("2023-10-01 08:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let end =
            NaiveDateTime::parse_from_str("2023-10-03 08:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let result1 = prices.calc_price(start, end, power).unwrap();
        println!("Calculated price for two days: {}", result1);
        let start =
            NaiveDateTime::parse_from_str("2023-10-01 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let end =
            NaiveDateTime::parse_from_str("2023-10-03 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let result2 = prices.calc_price(start, end, power).unwrap();
        println!("Calculated price for two days with midnight: {}", result2);
    }
}
