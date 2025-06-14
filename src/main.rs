use futures_util::SinkExt;
use futures_util::StreamExt;
use futures_util::stream::SplitSink;
use taranis::time::get_mock_now;
use tokio::net::TcpStream;
use tokio::sync::oneshot;
use tokio::time::Interval;
use tracing::Instrument;
use tracing::instrument;
use tracing_subscriber::fmt::time::ChronoLocal;
use tracing_subscriber::{EnvFilter, Layer};
use tracing_subscriber::{fmt::format::FmtSpan, layer::SubscriberExt, util::SubscriberInitExt};

use crossterm::event::{self, Event, KeyCode};
use tokio::task;

use tokio::time::{Duration, interval, interval_at, timeout};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

use taranis::charge::CHARGE;
use taranis::charge::Charge;
use taranis::conf::CONF;
use taranis::detail::ChargingDetail;
use taranis::message::{MSG, MessageType};

use tokio_tungstenite::tungstenite::Message as WsMessage;
type WsSender = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, WsMessage>;

/// 结束全局原子变量
static IS_CLOSED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[tokio::main]
async fn main() {
    // 打开日志文件
    let pid = std::process::id();
    let file_appender = tracing_appender::rolling::daily("logs", format!("app_{}.log", pid));
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // 设置日志过滤器
    let console_filter = if cfg!(debug_assertions) {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("info")
    };
    let file_filter = EnvFilter::new("trace");

    // 设置控制台日志格式
    let console_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stderr)
        .with_timer(ChronoLocal::new("%Y-%m-%d %H:%M:%S%.3f".to_string()))
        .with_ansi(true)
        .with_level(true)
        .with_target(false)
        .with_filter(console_filter);

    // 设置文件日志格式
    let file_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_span_events(FmtSpan::CLOSE | FmtSpan::NEW)
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_level(true)
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_filter(file_filter);

    // 初始化日志订阅者
    tracing_subscriber::registry()
        .with(console_layer)
        .with(file_layer)
        .init();

    work().await;
}

#[instrument]
/// 主工作函数，负责初始化充电桩，连接 WebSocket 服务器，并处理消息。
async fn work() {
    tracing::info!("程序 PID: {}", std::process::id());
    // 初始化充电桩
    tracing::info!("充电桩服务启动");
    let _conf = &*CONF;
    // 打断通道
    let (breakdown_tx, mut breakdown_rx) = oneshot::channel::<()>();
    // 检测是否允许充电桩被打断
    if CONF.charge.allow_break {
        tracing::info!("充电桩允许被打断, 按 'p' 键可以模拟充电桩损坏");
        wait_for_p_key(breakdown_tx).await;
    } else {
        tracing::info!("充电桩不允许被打断");
    }
    // 链接 WebSocket 服务器
    let result = timeout(
        Duration::from_secs(10),
        connect_async(CONF.websocket.url.clone()),
    )
    .await;
    let (ws_stream, _) = match result {
        Ok(Ok(val)) => val,
        Ok(Err(e)) => {
            tracing::error!("WebSocket 连接失败: {}", e);
            IS_CLOSED.store(true, std::sync::atomic::Ordering::Release);
            return;
        }
        Err(_) => {
            tracing::error!("WebSocket 连接超时");
            IS_CLOSED.store(true, std::sync::atomic::Ordering::Release);
            return;
        }
    };
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    tracing::info!("WebSocket 连接成功: {}", CONF.websocket.url);

    let mut update_tiker: Option<Interval> = None;
    let mut complete_tiker: Option<Interval> = None;

    // 注册充电桩
    register(&mut ws_sender).await;

    loop {
        tokio::select! {
            msg = ws_receiver.next() => {
                match msg {
                    Some(Ok(message)) => {
                        match message {
                            WsMessage::Text(text) => {
                                handle(text.to_string(), &mut ws_sender, &mut update_tiker, &mut complete_tiker).await;
                            }
                            WsMessage::Close(_) => {
                                tracing::info!(virtual_time = %get_mock_now(), "WebSocket 连接已关闭");
                                break;
                            }
                            _ => {
                                tracing::warn!(virtual_time = %get_mock_now(), "接收到非文本消息: {:?}，自动忽略", message);
                            }
                        }
                    }
                    Some(Err(e)) => {
                        tracing::error!(virtual_time = %get_mock_now(), "WebSocket 接收消息失败: {}", e);
                        break;
                    }
                    None => {
                        tracing::info!(virtual_time = %get_mock_now(), "WebSocket 连接已关闭");
                        break;
                    }
                }
            }
            _update = wait_opt_ticker(&mut update_tiker)=> {
                try_update_charge(&mut ws_sender, &mut update_tiker).await;
            }
            _complete = wait_opt_ticker(&mut complete_tiker) => {
                try_complete_charge(&mut ws_sender, &mut update_tiker, &mut complete_tiker).await;
            }
            _break = &mut breakdown_rx => {
                match _break {
                    Ok(_) => {
                        tracing::info!(virtual_time = %get_mock_now(), "接收到充电桩损坏信号");
                        try_breakdown_charge(&mut ws_sender, &mut update_tiker, &mut complete_tiker).await;
                        ws_sender.close().await.ok();
                        break;
                    }
                    Err(_) => {
                        tracing::warn!(virtual_time = %get_mock_now(), "充电桩损坏信号已被取消");
                        break;
                    }
                }
            }
        }
    }
    tracing::info!(virtual_time = %get_mock_now(), "充电桩服务已停止");
    IS_CLOSED.store(true, std::sync::atomic::Ordering::Release);
}

/// 等待一个可选的计时器，如果计时器存在，则等待其 tick，否则等待直到有新的事件发生。
async fn wait_opt_ticker(ticker: &mut Option<Interval>) {
    if let Some(t) = ticker {
        t.tick().await;
    } else {
        futures_util::future::pending::<()>().await;
    }
}

/// 设置计时器
fn set_ticker(ticker: &mut Option<Interval>, duration: Duration) {
    if duration.is_zero() {
        tracing::warn!(
            virtual_time = %get_mock_now(), "设置的计时器时长为零，将使用 tokio::time::interval (可能立即触发): {:?}",
            duration
        );
        // 对于零时长，如果期望立即触发，原始的 interval() 行为是符合的
        *ticker = Some(interval(duration));
    } else {
        // 计算第一个 tick 应该发生的时间
        tracing::debug!(virtual_time = %get_mock_now(), "设置计时器，间隔: {:?}", duration);
        let first_tick_time = tokio::time::Instant::now() + duration;
        *ticker = Some(interval_at(first_tick_time, duration));
    }
}

/// 移除计时器
fn remove_ticker(ticker: &mut Option<Interval>) {
    *ticker = None;
}

/// 等待 'p' 键被按下，如果允许充电桩被打断，则模拟充电桩损坏。
async fn wait_for_p_key(tx: oneshot::Sender<()>) {
    let _ = task::spawn_blocking(move || {
        loop {
            if event::poll(Duration::from_millis(100)).unwrap() {
                if let Event::Key(key_event) = event::read().unwrap() {
                    if key_event.code == KeyCode::Char('p') || key_event.code == KeyCode::Char('P')
                    {
                        tracing::info!("检测到 'p' 键被按下，模拟充电桩损坏");
                        let _ = tx.send(()); // 发送打断信号
                        break;
                    }
                }
            } else if IS_CLOSED.load(std::sync::atomic::Ordering::Acquire) {
                break;
            }
        }
    })
    .instrument(tracing::info_span!("等待 'p' 键被按下"));
}
/// 注册充电桩到 WebSocket 服务器
async fn register(ws_sender: &mut WsSender) {
    let reg_msg = MSG {
        type_: MessageType::Register,
        data: serde_json::to_string(&*CHARGE.lock().await).unwrap(),
    };
    match ws_sender
        .send(WsMessage::Text(
            serde_json::to_string(&reg_msg).unwrap().into(),
        ))
        .await
    {
        Ok(_) => tracing::info!(virtual_time = %get_mock_now(), "充电桩注册消息发送成功"),
        Err(e) => tracing::error!(virtual_time = %get_mock_now(), "充电桩注册消息发送失败: {}", e),
    }
}

/// 处理接收到的消息
async fn handle(
    message: String,
    ws_sender: &mut WsSender,
    update_ticker: &mut Option<Interval>,
    complete_ticker: &mut Option<Interval>,
) {
    static IS_CLOSED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

    tracing::debug!(virtual_time = %get_mock_now(), "接收到消息: {}", message);
    let msg: MSG = match serde_json::from_str(&message) {
        Ok(m) => m,
        Err(e) => {
            tracing::error!(virtual_time = %get_mock_now(), "消息解析失败: {}", e);
            return;
        }
    };

    match msg.type_ {
        MessageType::New => {
            if IS_CLOSED.load(std::sync::atomic::Ordering::SeqCst) {
                tracing::warn!(virtual_time = %get_mock_now(), "充电桩已关闭，无法处理新充电请求");
                return;
            }
            handle_new(msg.data, ws_sender, update_ticker, complete_ticker).await;
        }
        MessageType::Cancel => {
            if IS_CLOSED.load(std::sync::atomic::Ordering::SeqCst) {
                tracing::warn!(virtual_time = %get_mock_now(), "充电桩已关闭，无法取消充电");
                return;
            }
            handle_cancel(msg.data, ws_sender, update_ticker, complete_ticker).await
        }
        MessageType::Close => {
            if IS_CLOSED.load(std::sync::atomic::Ordering::SeqCst) {
                tracing::warn!(virtual_time = %get_mock_now(), "充电桩已关闭，无法再次关闭");
                return;
            }
            handle_close(ws_sender, update_ticker, complete_ticker).await;
            IS_CLOSED.store(true, std::sync::atomic::Ordering::SeqCst);
        }
        MessageType::Open => {
            if !IS_CLOSED.load(std::sync::atomic::Ordering::SeqCst) {
                tracing::warn!(virtual_time = %get_mock_now(), "充电桩未关闭，无法重新打开");
                return;
            }
            handle_open(update_ticker, complete_ticker).await;
            IS_CLOSED.store(false, std::sync::atomic::Ordering::SeqCst);
        }
        _ => {
            tracing::warn!(virtual_time = %get_mock_now(), "非法消息类型: {:?}", msg.type_);
        }
    }
}

/// 检查充电桩是否未工作，如果未工作且队列中有充电详单，则开始工作并设置计时器。
async fn not_working_check(charge: &mut Charge, complete_ticker: &mut Option<Interval>) -> bool {
    if !charge.is_working() && charge.get_queue_size() > 0 {
        tracing::info!(virtual_time = %get_mock_now(), "充电桩未工作，开始工作");
        charge.start_charging();
        // println!("{:?}", Duration::from_secs(charge.complete_interval()));
        set_ticker(
            complete_ticker,
            Duration::from_millis(charge.complete_interval()),
        );
        true
    } else {
        false
    }
}

/// 发送充电详单更新消息
async fn send_update(ws_sender: &mut WsSender, detail: &ChargingDetail) {
    let update_msg = MSG {
        type_: MessageType::Update,
        data: serde_json::to_string(detail).unwrap(),
    };
    match ws_sender
        .send(WsMessage::Text(
            serde_json::to_string(&update_msg).unwrap().into(),
        ))
        .await
    {
        Ok(_) => {
            tracing::debug!(virtual_time = %get_mock_now(), "充电详单更新消息发送成功: {}", detail.get_id())
        }
        Err(e) => {
            tracing::error!(virtual_time = %get_mock_now(), "充电详单更新消息发送失败: {}", e)
        }
    }
}

/// 发送充电详单完成消息
async fn send_complete(ws_sender: &mut WsSender, detail: &ChargingDetail) {
    let complete_msg = MSG {
        type_: MessageType::Complete,
        data: serde_json::to_string(detail).unwrap(),
    };
    match ws_sender
        .send(WsMessage::Text(
            serde_json::to_string(&complete_msg).unwrap().into(),
        ))
        .await
    {
        Ok(_) => tracing::info!(virtual_time = %get_mock_now(), "充电详单完成消息发送成功"),
        Err(e) => {
            tracing::error!(virtual_time = %get_mock_now(), "充电详单完成消息发送失败: {}", e)
        }
    }
}

/// 发送充电详单故障消息
async fn send_fault(ws_sender: &mut WsSender, detail: Option<&ChargingDetail>) {
    let fault_msg = MSG {
        type_: MessageType::Fault,
        data: serde_json::to_string(&detail).unwrap(),
    };
    match ws_sender
        .send(WsMessage::Text(
            serde_json::to_string(&fault_msg).unwrap().into(),
        ))
        .await
    {
        Ok(_) => tracing::info!(virtual_time = %get_mock_now(), "充电详单故障消息发送成功"),
        Err(e) => {
            tracing::error!(virtual_time = %get_mock_now(), "充电详单故障消息发送失败: {}", e)
        }
    }
}

/// 处理新的充电详单消息
async fn handle_new(
    msg: String,
    ws_sender: &mut WsSender,
    update_ticker: &mut Option<Interval>,
    complete_ticker: &mut Option<Interval>,
) {
    let detail: ChargingDetail = match serde_json::from_str(&msg) {
        Ok(d) => d,
        Err(e) => {
            tracing::error!(virtual_time = %get_mock_now(), "充电详单解析失败: {}", e);
            return;
        }
    };
    tracing::info!(virtual_time = %get_mock_now(), "接收到新的充电详单: {}", detail.get_id());

    if !detail.is_ready() {
        tracing::warn!(virtual_time = %get_mock_now(), "充电详单格式异常，无法加入队列");
        return;
    } else {
        let mut charge = CHARGE.lock().await;
        charge.add_detail(detail);
        tracing::info!(
            virtual_time = %get_mock_now(), "充电详单已加入队列，当前队列长度: {}",
            charge.get_queue_size()
        );
        if not_working_check(&mut charge, complete_ticker).await {
            send_update(ws_sender, charge.get_charging_detail_ref().unwrap()).await;
            set_ticker(
                update_ticker,
                Duration::from_millis(CONF.time.update_interval),
            );
        }
    }
}

/// 处理取消充电详单消息
async fn handle_cancel(
    msg: String,
    ws_sender: &mut WsSender,
    update_ticker: &mut Option<Interval>,
    complete_ticker: &mut Option<Interval>,
) {
    let detail: ChargingDetail = match serde_json::from_str(&msg) {
        Ok(d) => d,
        Err(e) => {
            tracing::error!(virtual_time = %get_mock_now(), "充电详单解析失败: {}", e);
            return;
        }
    };
    let detail_id = detail.get_id();
    tracing::info!(virtual_time = %get_mock_now(), "接收到取消充电详单请求: {}", detail_id);

    let mut charge = CHARGE.lock().await;
    match charge.cancel_charging(detail_id) {
        Ok(detail) => {
            tracing::info!(virtual_time = %get_mock_now(), "充电详单 {} 已取消", detail_id);
            send_update(ws_sender, &detail).await;
            if not_working_check(&mut charge, complete_ticker).await {
                send_update(ws_sender, charge.get_charging_detail_ref().unwrap()).await;
                set_ticker(
                    update_ticker,
                    Duration::from_millis(CONF.time.update_interval),
                );
            }
        }
        Err(e) => {
            tracing::error!(virtual_time = %get_mock_now(), "取消充电详单失败: {}", e);
        }
    }
}

/// 处理关闭充电桩请求
async fn handle_close(
    ws_sender: &mut WsSender,
    update_ticker: &mut Option<Interval>,
    complete_ticker: &mut Option<Interval>,
) {
    tracing::info!(virtual_time = %get_mock_now(), "接收到关闭充电桩请求");
    let mut charge = CHARGE.lock().await;
    if let Some(detail) = charge.close() {
        tracing::info!(virtual_time = %get_mock_now(), "充电桩已关闭，当前被打断的充电详单: {}", detail.get_id());
        send_update(ws_sender, &detail).await;
    } else {
        tracing::info!(virtual_time = %get_mock_now(), "充电桩队列为空，没有被打断的充电详单");
    }
    remove_ticker(update_ticker);
    remove_ticker(complete_ticker);
}

/// 处理打开充电桩请求
async fn handle_open(update_ticker: &mut Option<Interval>, complete_ticker: &mut Option<Interval>) {
    tracing::info!(virtual_time = %get_mock_now(), "接收到打开充电桩请求");
    remove_ticker(update_ticker);
    remove_ticker(complete_ticker);
}

/// 尝试更新充电状态
async fn try_update_charge(ws_sender: &mut WsSender, update_ticker: &mut Option<Interval>) {
    let mut charge = CHARGE.lock().await;
    if charge.is_working() {
        charge.update_charging();
        if let Some(detail) = charge.get_charging_detail_ref() {
            send_update(ws_sender, detail).await;
        } else {
            unreachable!(
                "It should never happen that there is no charging detail when the charge is working"
            );
        }
    } else {
        tracing::error!(virtual_time = %get_mock_now(), "充电桩未处于工作状态，无法更新充电状态");
        remove_ticker(update_ticker);
    }
}

/// 尝试完成充电
async fn try_complete_charge(
    ws_sender: &mut WsSender,
    update_ticker: &mut Option<Interval>,
    complete_ticker: &mut Option<Interval>,
) {
    let mut charge = CHARGE.lock().await;
    if charge.is_working() {
        if let Some(detail) = charge.complete_charging() {
            send_complete(ws_sender, &detail).await;
            remove_ticker(complete_ticker);
            remove_ticker(update_ticker);
            tracing::info!(virtual_time = %get_mock_now(), "充电详单 {} 已完成", detail.get_id());
            if not_working_check(&mut charge, complete_ticker).await {
                send_update(ws_sender, charge.get_charging_detail_ref().unwrap()).await;
                set_ticker(
                    update_ticker,
                    Duration::from_millis(CONF.time.update_interval),
                );
            }
        } else {
            unreachable!(
                "It should never happen that there is no charging detail when the charge is working"
            );
        }
    } else {
        tracing::error!(virtual_time = %get_mock_now(), "充电桩未处于工作状态，无法完成充电");
        remove_ticker(complete_ticker);
        remove_ticker(update_ticker);
    }
}

/// 尝试打断充电
async fn try_breakdown_charge(
    ws_sender: &mut WsSender,
    update_ticker: &mut Option<Interval>,
    complete_ticker: &mut Option<Interval>,
) {
    tracing::error!(virtual_time = %get_mock_now(),"充电桩损坏");
    let mut charge = CHARGE.lock().await;
    if charge.is_working() {
        if let Some(detail) = charge.breakdown() {
            send_fault(ws_sender, Some(&detail)).await;
            remove_ticker(complete_ticker);
            remove_ticker(update_ticker);
            tracing::info!(virtual_time = %get_mock_now(), "充电详单 {} 已被打断", detail.get_id());
        } else {
            unreachable!(
                "It should never happen that there is no charging detail when the charge is working"
            );
        }
    } else {
        tracing::info!(virtual_time = %get_mock_now(), "充电桩未处于工作状态，没有被打断的充电详单");
        send_fault(ws_sender, None).await;
        remove_ticker(complete_ticker);
        remove_ticker(update_ticker);
    }
}
