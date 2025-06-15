use futures_util::{SinkExt, StreamExt};
use taranis::{
    conf::CONF,
    detail::ChargingDetail,
    message::{MSG, MessageType},
};
use tokio::{net::TcpListener, time::sleep};
use tokio_tungstenite::tungstenite::Message;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = CONF.websocket.url.clone();

    let addr = url
        .strip_prefix("ws://")
        .or_else(|| url.strip_prefix("wss://"))
        .expect("Invalid WebSocket URL format")
        .to_string();

    // Create the event loop and TCP listener we'll accept connections on.
    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");
    println!("Listening on: {}", addr);

    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(async move {
            let ws_stream = tokio_tungstenite::accept_async(stream)
                .await
                .expect("Error during the websocket handshake occurred");

            println!(
                "New websocket connection: {}",
                ws_stream.get_ref().peer_addr().unwrap()
            );

            let (mut outgoing, mut incoming) = ws_stream.split();

            let mut detail_id = 0;

            while let Some(result) = incoming.next().await {
                match result {
                    Ok(message) => {
                        // println!("Received: {:?}", message);
                        if message.is_text() {
                            // println!("Text message: {}", message.to_text().unwrap());
                            let msg: MSG = serde_json::from_str(message.to_text().unwrap())
                                .expect(format!("Failed to parse message: {:?}", message).as_str());
                            if msg.type_ == MessageType::Register {
                                println!("Register message received: {:?}", msg);
                                sleep(std::time::Duration::from_secs(5)).await;
                                // Here you can handle the register message as needed
                                // For example, you might want to send a response back
                                for _ in 0..CONF.charge.size {
                                    let detail = ChargingDetail::test_new(detail_id);
                                    detail_id += 1;
                                    let response = MSG {
                                        type_: MessageType::New,
                                        data: serde_json::to_string(&detail).unwrap(),
                                    };
                                    outgoing
                                        .send(Message::Text(
                                            serde_json::to_string(&response).unwrap().into(),
                                        ))
                                        .await
                                        .unwrap();
                                }
                            } else if msg.type_ == MessageType::Complete {
                                let detail: Option<ChargingDetail> =
                                    serde_json::from_str(&msg.data).ok();
                                if let Some(detail) = detail {
                                    println!(
                                        "Charging Detail Completed: {}",
                                        serde_json::to_string_pretty(&detail).unwrap()
                                    );
                                    let new_detail = ChargingDetail::test_new(detail_id);
                                    detail_id += 1;
                                    let response = MSG {
                                        type_: MessageType::New,
                                        data: serde_json::to_string(&new_detail).unwrap(),
                                    };
                                    outgoing
                                        .send(Message::Text(
                                            serde_json::to_string(&response).unwrap().into(),
                                        ))
                                        .await
                                        .unwrap();
                                } else {
                                    println!("detail is None or invalid format");
                                }
                            } else {
                                println!("MSG type: {:?}", msg.type_);
                                let detail: Option<ChargingDetail> =
                                    serde_json::from_str(&msg.data).ok();
                                if let Some(detail) = detail {
                                    println!(
                                        "Charging Detail: {}",
                                        serde_json::to_string_pretty(&detail).unwrap()
                                    );
                                    // Here you can handle the ChargingDetail as needed
                                } else {
                                    println!("detail is None or invalid format");
                                }
                            }
                        } else if message.is_binary() {
                            println!("Binary message: {:?}", message.into_data());
                        } else if message.is_ping() {
                            println!("Ping received, sending Pong.");
                            outgoing.send(Message::Pong("Pong!".into())).await.unwrap();
                        } else if message.is_close() {
                            println!("Close message received, closing connection.");
                            outgoing
                                .send(Message::Close(None))
                                .await
                                .unwrap_or_else(|e| {
                                    println!("Error sending close message: {:?}", e);
                                });
                            break;
                        }
                    }
                    Err(e) => {
                        println!("Error receiving message: {:?}", e);
                        break;
                    }
                }
            }

            println!("Websocket connection closed.");
        });
    }

    Ok(())
}
