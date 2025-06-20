use crate::conf::{CONF, ChargeType};
use crate::detail::ChargingDetail;
use crate::price::calc_price_with_tz;
use crate::time::get_mock_now;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
/// 充电桩结构体
pub struct Charge {
    /// 充电桩ID
    charge_id: Uuid,
    #[serde(rename = "type")]
    /// 充电类型
    type_: ChargeType,
    /// 充电功率，单位为kW
    power: f64,
    /// 队列大小
    size: u32,
    #[serde(skip)]
    /// 充电详单队列
    queue: Vec<ChargingDetail>,
    #[serde(skip)]
    /// 是否正在工作
    working: bool,
}

impl Charge {
    /// 创建一个新的充电桩实例
    pub fn new(type_: ChargeType, power: f64, size: u32) -> Self {
        Charge {
            charge_id: Uuid::new_v4(),
            type_,
            power,
            size,
            queue: Vec::with_capacity(size as usize),
            working: false,
        }
    }

    /// 添加充电详单到充电桩队列
    pub fn add_detail(&mut self, detail: ChargingDetail) {
        if detail.get_type() != self.type_ {
            tracing::warn!(
                virtual_time = %get_mock_now(),
                "充电详单类型不匹配，无法添加到充电桩队列: {:?} != {:?}",
                detail.get_type(),
                self.type_
            );
            return;
        } else if self.queue.len() < self.size as usize {
            self.queue.push(detail);
        } else {
            tracing::warn!("充电桩队列已满，无法添加新的充电详单");
        }
    }

    /// 开始充电
    pub fn start_charging(&mut self) {
        if self.queue.is_empty() {
            tracing::warn!(virtual_time = %get_mock_now(), "充电桩队列为空，无法开始充电");
            return;
        }
        if self.working {
            tracing::warn!(virtual_time = %get_mock_now(), "充电桩正在工作，无法再次开始充电");
            return;
        }

        self.working = true; // 设置充电桩为工作状态

        let detail = self.queue.first_mut().unwrap();

        detail.start(get_mock_now());

        tracing::info!(
            virtual_time = %get_mock_now(),
            "充电桩开始充电 详单 ID: {}",
            detail.get_id(),
        );
    }

    /// 更新充电状态
    pub fn update_charging(&mut self) {
        if self.queue.is_empty() {
            tracing::warn!(virtual_time = %get_mock_now(), "充电桩队列为空，无法更新充电状态");
            return;
        }
        if !self.working {
            tracing::warn!(virtual_time = %get_mock_now(), "充电桩未处于工作状态，无法更新充电状态");
            return;
        }

        let detail = self.queue.first_mut().unwrap();
        let now = get_mock_now();
        let cost = calc_price_with_tz(detail.clone_start_time(), now.clone(), self.power).unwrap();
        detail.update_state(
            already_charged(self.power, &detail, now.clone()),
            cost.0,
            cost.1,
            now.clone(),
        );
    }

    /// 完成充电
    pub fn complete_charging(&mut self) -> Option<ChargingDetail> {
        // 检查队列是否为空或充电桩是否处于工作状态
        // 如果队列为空或充电桩未工作，返回 None
        if self.queue.is_empty() {
            tracing::warn!(virtual_time = %get_mock_now(), "充电桩队列为空，无法完成充电");
            None
        } else if !self.working {
            tracing::warn!(virtual_time = %get_mock_now(), "充电桩未处于工作状态，无法完成充电");
            None
        } else {
            let mut detail = self.queue.remove(0);
            self.working = false; // 完成充电时设置充电桩为非工作状态
            let now = get_mock_now();
            let cost =
                calc_price_with_tz(detail.clone_start_time(), now.clone(), self.power).unwrap();
            detail.complete(
                already_charged(self.power, &detail, now.clone()),
                cost.0,
                cost.1,
                now.clone(),
            );
            Some(detail)
        }
    }

    /// 取消充电
    pub fn cancel_charging(&mut self, detail_id: u32) -> Result<ChargingDetail, String> {
        if let Some(pos) = self.queue.iter().position(|d| d.get_id() == detail_id) {
            let detail = self.queue.get_mut(pos).unwrap();
            let now = get_mock_now();
            if pos == 0 {
                let cost =
                    calc_price_with_tz(detail.clone_start_time(), now.clone(), self.power).unwrap();
                detail.interrupt(
                    already_charged(self.power, &detail, now.clone()),
                    cost.0,
                    cost.1,
                    now.clone(),
                );
                self.working = false; // 取消充电时设置充电桩为非工作状态
            } else {
                detail.interrupt(
                    already_charged(self.power, &detail, now.clone()),
                    0.0,
                    0.0,
                    now.clone(),
                );
            }
            Ok(self.queue.remove(pos))
        } else {
            tracing::warn!(virtual_time = %get_mock_now(), "未找到指定的充电详单，无法取消充电");
            Err("no such charging detail".to_string())
        }
    }

    /// 获取正在充电的充电详单的引用
    pub fn get_charging_detail_ref(&self) -> Option<&ChargingDetail> {
        self.queue.first()
    }

    /// 关闭充电桩
    pub fn close(&mut self) -> Option<ChargingDetail> {
        self.working = false; // 设置充电桩为非工作状态
        if self.queue.is_empty() {
            tracing::info!(virtual_time = %get_mock_now(), "充电桩队列为空，没有被打断的充电详单");
            None
        } else {
            let mut detail = self.queue.remove(0);
            self.queue.clear(); // 清空队列
            let now = get_mock_now();
            let cost =
                calc_price_with_tz(detail.clone_start_time(), now.clone(), self.power).unwrap();
            detail.interrupt(
                already_charged(self.power, &detail, now.clone()),
                cost.0,
                cost.1,
                now.clone(),
            );
            Some(detail)
        }
    }

    /// 损坏充电桩
    pub fn breakdown(&mut self) -> Option<ChargingDetail> {
        self.close() // 关闭充电桩并清空队列
    }

    /// 是否正在工作
    pub fn is_working(&self) -> bool {
        self.working
    }

    /// 获取队列大小
    pub fn get_queue_size(&self) -> usize {
        self.queue.len()
    }

    /// 获取预计完成间隔(毫秒)
    pub fn complete_interval(&self) -> u64 {
        if self.queue.is_empty() {
            tracing::warn!(virtual_time = %get_mock_now(), "充电桩队列为空，无法获取完成间隔");
            0
        } else if !self.working {
            tracing::warn!(virtual_time = %get_mock_now(), "充电桩未处于工作状态，无法获取完成间隔");
            0
        } else {
            let time = self
                .queue
                .first()
                .unwrap()
                .get_estimated_end_time(self.power);
            if let Some(end_time) = time {
                let now = get_mock_now();
                let duration = end_time.signed_duration_since(now);
                let millis = duration.num_milliseconds() + 100; // 加100毫秒以避免精度问题
                millis as u64 / CONF.time.speed // 考虑加速倍数
            } else {
                tracing::warn!(virtual_time = %get_mock_now(), "无法计算预计充电结束时间");
                0
            }
        }
    }
}

fn already_charged(
    power: f64,
    detail: &ChargingDetail,
    time: chrono::DateTime<chrono::Utc>,
) -> f64 {
    let start_time = detail.clone_start_time();
    let duration = time.signed_duration_since(start_time);
    let hours = duration.num_seconds() as f64 / 3600.0; // 转换为小时
    hours * power // 计算已充电度数
}

/// 全局充电桩实例，使用 Lazy 和 Mutex 确保线程安全和延迟初始化
pub static CHARGE: Lazy<Mutex<Charge>> = Lazy::new(|| {
    Mutex::new(Charge::new(
        CONF.charge.charge_type,
        CONF.charge.power,
        CONF.charge.size,
    ))
});

#[cfg(test)]
mod test {
    use super::*;
    use crate::conf::ChargeType;

    #[test]
    fn test_charge_serialization() {
        // v4 生成
        let charge = Charge {
            charge_id: Uuid::new_v4(),
            type_: ChargeType::Fast,
            power: 30.0,
            size: 5,
            queue: vec![],
            working: false,
        };

        let serialized = serde_json::to_string_pretty(&charge).unwrap();
        println!("Serialized Charge: {}", serialized);

        let deserialized: Charge = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.charge_id, charge.charge_id);
        assert_eq!(deserialized.type_, charge.type_);
        assert_eq!(deserialized.power, charge.power);
        assert_eq!(deserialized.size, charge.size);
        assert_eq!(deserialized.queue.len(), charge.queue.len());
    }
}
