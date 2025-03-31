use futures::{SinkExt, StreamExt};
use game::{Client, Game, Games, SoccerGame};
use message::{MessageType, SoccerMoveMessage, WsMessage};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tokio_tungstenite::{accept_async, accept_hdr_async, tungstenite::protocol::Message};
use url;

mod game;
mod math;
mod message;
mod physics;

static NEXT_ID: AtomicUsize = AtomicUsize::new(1);
static NEXT_GAME: AtomicUsize = AtomicUsize::new(2);
struct ConnectionInfo {
    auth_token: Option<String>,
    game: Option<usize>,
    playerIndex: usize,
}
#[tokio::main]
async fn main() {
    let games: Games = Arc::new(RwLock::new(HashMap::new()));

    let port = "127.0.0.1:8080".to_string();
    let addr: SocketAddr = port.parse().expect("Invalid Address");

    let listener = TcpListener::bind(addr).await.expect("Failed to bind");

    println!("Listening on {}", addr);
    // 60hz
    #[allow(unused_must_use)] // suppress warning
    start_periodic_task(games.clone(), Duration::from_millis(1000 / 60));

    while let Ok((stream, _)) = listener.accept().await {
        let games = games.clone();

        tokio::spawn(async move {
            handle_connection(stream, games).await;
        });
    }
}
async fn start_periodic_task(games: Games, duration: Duration) {
    let mut interval = interval(duration);
    loop {
        interval.tick().await;
        handle_frame(games.clone()).await;
    }
}
async fn handle_frame(games: Games) {
    let read = games.read().await;
    for (_, value) in read.iter() {
        value.write().await.update();
    }
}

async fn handle_connection(stream: TcpStream, games: Games) {
    let client_id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let mut conn_info = ConnectionInfo {
        auth_token: None,
        game: None,
        playerIndex: 0,
    };
    let mut client = Client::new(client_id);
    let ws_stream = match accept_hdr_async(
        stream,
        |req: &tokio_tungstenite::tungstenite::http::Request<()>,
         res: tokio_tungstenite::tungstenite::http::Response<()>| {
            conn_info.auth_token = req
                .headers()
                .get("Authorization")
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string());
            match req.uri().query() {
                Some(query) => {
                    let query_params = parse_query_params(query);
                    conn_info.game = query_params
                        .get("game")
                        .and_then(|s| s.parse::<usize>().ok());
                }
                None => (),
            }
            Ok(res)
        },
    )
    .await
    {
        Ok(ws) => ws,
        Err(e) => {
            println!("Error during the websocket handshake: {}", e);
            return;
        }
    };

    if let Some(auth_token) = &conn_info.auth_token {
        // make authorization fetch request herer
    } else {
        println!("Authorization token not provided, skipping for testing");
        // return;
    }
    let game_id = match &conn_info.game {
        Some(id) => {
            conn_info.playerIndex = 1; // we've found a game, so player 1
            *id
        }
        None => {
            // Create new game with write lock
            let mut games_write = games.write().await;
            let new_id = games_write.keys().max().copied().unwrap_or(0) + 1;
            games_write.insert(
                new_id,
                Arc::new(RwLock::new(Game::new(SoccerGame::new(), Vec::new()))),
            );
            new_id
        }
    };

    // Now get the read lock once and keep it in scope
    let games_guard = games.read().await;
    let game = match games_guard.get(&game_id) {
        Some(game) => game,
        None => {
            println!("Game not found after creation");
            return;
        }
    };

    let (mut sender, mut receiver) = ws_stream.split();
    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Binary(data)) => {
                if let Some(ws_msg) = WsMessage::from_bytes(&data) {
                    match ws_msg.msg_type {
                        MessageType::Ping => {
                            client.update_ping();
                            let response = WsMessage {
                                msg_type: MessageType::Pong,
                                payload: vec![],
                            };
                            sender
                                .send(Message::Binary(response.to_bytes()))
                                .await
                                .unwrap();
                        }
                        MessageType::State => {
                            let deserialized_game = game.read().await;
                            if let Some(soccer_game) = deserialized_game.downcast::<SoccerGame>() {
                                let response = WsMessage {
                                    msg_type: MessageType::State,
                                    payload: soccer_game.to_bytes(),
                                };
                                sender
                                    .send(Message::Binary(response.to_bytes()))
                                    .await
                                    .unwrap();
                            } else {
                                eprintln!("Failed to downcast to SoccerGame");
                            }
                        }
                        MessageType::SoccerMove => {
                            let soccer_move_message =
                                match bincode::deserialize::<SoccerMoveMessage>(&ws_msg.payload)
                                    .ok()
                                {
                                    Some(message) => message,
                                    None => {
                                        break;
                                    }
                                };
                            let mut game_lock = game.write().await;
                            if let Some(soccer_game) = game_lock.downcast_mut::<SoccerGame>() {
                                soccer_game.pucks[conn_info.playerIndex].vx =
                                    soccer_move_message.vx;
                                soccer_game.pucks[conn_info.playerIndex].vy =
                                    soccer_move_message.vy;
                            }
                        }
                        _ => {
                            println!("Received message type: {:?}", ws_msg.msg_type);
                        }
                    }
                }
            }
            Ok(Message::Close(_)) => break,
            Ok(_) => (),
            Err(e) => {
                println!("Error processing message: {}", e);
                break;
            }
        }
    }
}

fn parse_query_params(query: &str) -> HashMap<String, String> {
    url::form_urlencoded::parse(query.as_bytes())
        .into_owned()
        .collect()
}
