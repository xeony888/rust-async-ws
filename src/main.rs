use futures::{SinkExt, StreamExt};
use game::{Client, Game, GameLogic, Games, SoccerGame};
use message::{MessageType, SoccerMoveMessage, WsMessage};
use nalgebra::vector;
use num_cpus;
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
use sysinfo::System;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tokio_tungstenite::{accept_hdr_async, tungstenite::protocol::Message};
use url;
mod game;
mod message;

static NEXT_ID: AtomicUsize = AtomicUsize::new(1);

struct ConnectionInfo {
    auth_token: Option<String>,
    game: Option<usize>,
    name: Option<String>,
    player_index: usize,
}
#[tokio::main]
async fn main() {
    let mut sys = System::new_all();
    sys.refresh_all();

    let total_memory = sys.total_memory(); // In kilobytes
    let available_memory = sys.free_memory(); // In kilobytes
    let physical_cores = num_cpus::get_physical();
    let logical_threads = num_cpus::get();

    println!("Total Memory: {} MB", total_memory / 1024);
    println!("Available Memory: {} MB", available_memory / 1024);
    println!("Physical Cores: {}", physical_cores);
    println!("Logical Threads: {}", logical_threads);

    let games: Games = Arc::new(RwLock::new(HashMap::new()));

    let port = "127.0.0.1:8080".to_string();
    let addr: SocketAddr = port.parse().expect("Invalid Address");

    let listener = TcpListener::bind(addr).await.expect("Failed to bind");

    println!("Listening on {}", addr);
    // 60hz
    tokio::spawn(start_periodic_task(
        games.clone(),
        Duration::from_millis(1000 / 60),
    ));
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
        name: None,
        player_index: 0,
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
                    conn_info.name = query_params.get("name").cloned();
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
    // this name param should be fetched from the server once we are connected
    if let Some(name) = &conn_info.name {
    } else {
        // uh oh
        println!("User name not found")
    }
    // name is a temp param before authorization is completed
    if let Some(auth_token) = &conn_info.auth_token {
        // make authorization fetch request herer
    } else {
        // println!("Authorization token not provided, skipping for testing");
        // return;
    }
    let game_id = match &conn_info.game {
        Some(id) => *id,
        None => {
            let mut games = games.write().await;

            // Find first available game (async-compatible loop)
            let open_id = {
                let mut found_id = None;
                for (&id, game) in games.iter() {
                    let mut g = game.write().await;
                    let name = conn_info.name.clone().unwrap();
                    if g.players.contains(&name) {
                        found_id = Some(id);
                        println!("Found game {} for player {}", id, name);
                    }
                }
                match found_id {
                    Some(_) => found_id,
                    None => {
                        for (&id, game) in games.iter() {
                            let mut g = game.write().await;
                            if g.players.len() == 1 {
                                found_id = Some(id);
                                println!(
                                    "Player {} joined game {}",
                                    conn_info.name.clone().unwrap(),
                                    found_id.clone().unwrap()
                                );
                                g.players.push(conn_info.name.clone().unwrap());
                                break;
                            }
                        }
                        found_id
                    }
                }
            };

            if let Some(id) = open_id {
                id
            } else {
                let new_id = games.keys().max().copied().unwrap_or(0) + 1;
                let player_name = conn_info.name.clone().unwrap();
                println!("Player {} created game {}", player_name, new_id);
                games.insert(
                    new_id,
                    Arc::new(RwLock::new(Game::new(
                        SoccerGame::new(),
                        vec![player_name], // Use the cloned value here
                    ))),
                );
                new_id
            }
        }
    };
    // Now get the read lock once and keep it in scope
    let name = conn_info.name.clone().unwrap();
    let (game, player_index) = {
        let games_guard = games.read().await;
        match games_guard.get(&game_id) {
            Some(g) => {
                println!(
                    "Game {} joined with {} players",
                    game_id,
                    g.read().await.players.len()
                );
                (
                    Arc::clone(g),
                    g.read()
                        .await
                        .players
                        .iter()
                        .position(|s| *s == name)
                        .unwrap(),
                )
            } // Clone the Arc to keep access
            None => {
                println!("Game not found");
                return;
            }
        }
    };
    conn_info.player_index = player_index;

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
                                let index = conn_info.player_index * 5
                                    + soccer_move_message.target as usize;
                                soccer_game.bodies[soccer_game.pucks[index]].set_linvel(
                                    vector![soccer_move_message.vx, soccer_move_message.vy],
                                    true,
                                );
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
    let game_read = game.read().await;
    if game_read.players.len() == 1 {
        games.write().await.remove(&game_id);
        println!("Removed game {game_id} because last player disconnected");
    }
}

fn parse_query_params(query: &str) -> HashMap<String, String> {
    url::form_urlencoded::parse(query.as_bytes())
        .into_owned()
        .collect()
}
