use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{conf::{ChargeType, CONF}, price::round_to_precision};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
enum ChargeStatus {
    #[serde(rename = "waiting")]
    /// 充电等待中
    Waiting,
    #[serde(rename = "charging")]
    /// 充电中
    Charging,
    #[serde(rename = "completed")]
    /// 充电完成
    Completed,
    #[serde(rename = "interrupted")]
    /// 充电中断
    Interrupted,
}

#[derive(Serialize, Deserialize, Clone)]
/// 充电详单
pub struct ChargingDetail {
    /// 充电详单ID
    id: u32,
    /// 充电请求度数
    request_amount: f64,
    #[serde(rename = "type")]
    /// 充电类型
    type_: ChargeType,
    /// 已经充电度数
    already_charged: f64,
    /// 充电开始时间
    start_time: Option<DateTime<Utc>>,
    /// 充电最后更新时间
    last_update_time: Option<DateTime<Utc>>,
    /// 充电结束时间
    end_time: Option<DateTime<Utc>>,
    /// 充电费用
    charge_cost: f64,
    /// 服务费
    service_fee: f64,
    /// 总费用
    total_cost: f64,
    /// 充电状态
    status: ChargeStatus,
}

impl ChargingDetail {

    pub fn test_new(id: u32) -> Self {
        ChargingDetail {
            id: id,
            request_amount: 30.0,
            type_: CONF.charge.charge_type,
            already_charged: 0.0,
            start_time: None,
            last_update_time: None,
            end_time: None,
            charge_cost: 0.0,
            service_fee: 0.0,
            total_cost: 0.0,
            status: ChargeStatus::Waiting,
        }
    }

    /// 判断充电详单是否已准备好
    pub fn is_ready(&self) -> bool {
        return self.already_charged == 0.0
            && self.start_time.is_none()
            && self.last_update_time.is_none()
            && self.end_time.is_none()
            && self.charge_cost == 0.0
            && self.service_fee == 0.0
            && self.total_cost == 0.0
            && self.status == ChargeStatus::Waiting;
    }

    /// 启动充电详单
    pub fn start(&mut self, time: DateTime<Utc>) {
        if self.status != ChargeStatus::Waiting {
            tracing::error!("无法在非等待状态下开始充电详单");
            panic!("Cannot start charging details when not in waiting state");
        }
        self.start_time = Some(time);
        self.last_update_time = Some(time);
        self.status = ChargeStatus::Charging;
    }

    /// 更新充电详单状态
    pub fn update_state(&mut self, already_charged: f64, charge_cost: f64, service_fee: f64, time: DateTime<Utc>) {
        if self.status != ChargeStatus::Charging {
            tracing::error!("无法在非充电状态下更新充电详单");
            panic!("Cannot update charging details when not in charging state");
        }
        self.last_update_time = Some(time);
        self.already_charged = already_charged;
        self.charge_cost = charge_cost;
        self.service_fee = service_fee;
        self.total_cost = round_to_precision(charge_cost + service_fee, 2);
    }

    /// 完成充电详单
    pub fn complete(&mut self, already_charged: f64, charge_coost: f64, service_fee: f64, time: DateTime<Utc>) {
        if self.status != ChargeStatus::Charging {
            tracing::error!("无法在非充电状态下完成充电详单");
            panic!("Cannot complete charging details when not in charging state");
        }
        self.last_update_time = Some(time);
        self.already_charged = already_charged;
        self.charge_cost = charge_coost;
        self.service_fee = service_fee;
        self.total_cost = round_to_precision(charge_coost + service_fee, 2);
        self.end_time = Some(time);
        self.status = ChargeStatus::Completed;
    }

    /// 中断充电详单
    pub fn interrupt(&mut self, already_charged: f64, charge_coost: f64, service_fee: f64, time: DateTime<Utc>) {
        if self.status != ChargeStatus::Charging && self.status != ChargeStatus::Waiting {
            tracing::error!("无法在除充电或等待外状态下中断充电详单");
            panic!("Cannot interrupt charging details when not in charging or waiting state");
        }
        self.last_update_time = Some(time);
        self.already_charged = already_charged;
        self.charge_cost = charge_coost;
        self.service_fee = service_fee;
        self.total_cost = round_to_precision(charge_coost + service_fee, 2);
        self.status = ChargeStatus::Interrupted;
    }

    /// 获取充电详单的起始时间
    pub fn clone_start_time(&self) -> DateTime<Utc> {
        if self.status == ChargeStatus::Waiting{
            tracing::error!("无法在等待状态下获取充电开始时间");
            panic!("Cannot get start time when not in charging state");
        }
        self.start_time.clone().unwrap()
    }

    /// 获取充电详单的ID
    pub fn get_id(&self) -> u32 {
        self.id
    }

    /// 获取预计充电结束时间
    pub fn get_estimated_end_time(&self, power: f64) -> Option<DateTime<Utc>> {
        if self.status != ChargeStatus::Charging {
            tracing::error!("无法在非充电状态下获取预计充电结束时间");
            return None;
        }
        let remaining_amount = self.request_amount - self.already_charged;
        let estimated_duration = remaining_amount / power; // 假设 power 是单位时间内充电的度数
        Some(self.start_time.unwrap() + chrono::Duration::seconds((estimated_duration * 3600.0) as i64))
    }

    /// 获取充电详单的类型
    pub fn get_type(&self) -> ChargeType {
        self.type_
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialization() {
        let details = ChargingDetail {
            id: 1,
            request_amount: 100.0,
            type_: ChargeType::Fast,
            already_charged: 50.0,
            start_time: Some(Utc::now()),
            last_update_time: Some(Utc::now()),
            end_time: None,
            charge_cost: 10.0,
            service_fee: 2.0,
            total_cost: 12.0,
            status: ChargeStatus::Charging,
        };

        let serialized = serde_json::to_string_pretty(&details).unwrap();
        println!("Serialized JSON:\n{}", serialized);

        let deserialized: ChargingDetail = serde_json::from_str(&serialized).unwrap();
        assert_eq!(details.id, deserialized.id);
        assert_eq!(details.request_amount, deserialized.request_amount);
    }
}
