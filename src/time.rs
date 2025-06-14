use std::sync::LazyLock;

use chrono::{DateTime, Utc};

use crate::conf::CONF;

static REAL_START_TIME: LazyLock<DateTime<Utc>> =
    LazyLock::new(|| Utc::now());

static MOCK_TIME: LazyLock<DateTime<Utc>> =
    LazyLock::new(|| CONF.time.start_time.unwrap_or_else(|| REAL_START_TIME.clone()));

/// 获取当前时间(精确到毫秒)
pub fn get_mock_now() -> DateTime<Utc> {
    if CONF.time.speed == 1 {
        // 如果加速倍数为1，直接返回当前时间
        Utc::now()
    } else {
        // 计算从开始时间到现在的时间差
        let elapsed = Utc::now().signed_duration_since(*REAL_START_TIME);
        // 根据加速倍数计算新的时间
        let accelerated_duration = elapsed.num_milliseconds() * CONF.time.speed as i64;
        // 返回加速后的时间
        *MOCK_TIME + chrono::Duration::milliseconds(accelerated_duration)
    }
}