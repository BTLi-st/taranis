use std::sync::LazyLock;

use chrono::{DateTime, Duration, Utc};

use crate::conf::CONF;

static REAL_START_TIME: LazyLock<DateTime<Utc>> = LazyLock::new(|| Utc::now());

static MOCK_TIME: LazyLock<DateTime<Utc>> =
    LazyLock::new(|| CONF.time.start_time.unwrap_or_else(|| Utc::now()));

/// 获取当前时间(精确到毫秒)
pub fn get_mock_now() -> DateTime<Utc> {
    if CONF.time.speed == 1 {
        // 如果加速倍数为1，直接返回当前时间
        Utc::now()
    } else {
        // 计算从开始时间到现在的时间差
        let elapsed = Utc::now().signed_duration_since(*REAL_START_TIME);
        let duration_nanos = elapsed.num_nanoseconds();
        if let Some(nanos) = duration_nanos {
            // 计算加速后的时间(精确到纳秒)
            let accelerated_duration = Duration::nanoseconds(nanos * CONF.time.speed as i64);
            *MOCK_TIME + accelerated_duration
        } else {
            let duration_micros = elapsed.num_microseconds();
            if let Some(micros) = duration_micros {
                // 计算加速后的时间(精确到微秒)
                let accelerated_duration = Duration::microseconds(micros * CONF.time.speed as i64);
                *MOCK_TIME + accelerated_duration
            } else {
                // 如果纳秒和微秒都为 None，使用毫秒
                let duration_mullis = elapsed.num_milliseconds();
                let accelerated_duration =
                    Duration::milliseconds(duration_mullis * CONF.time.speed as i64);
                *MOCK_TIME + accelerated_duration
            }
        }
    }
}
