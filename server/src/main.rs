use anyhow::{anyhow, Context};
use futures_util::lock::Mutex;
use futures_util::{stream::SplitSink, SinkExt};
use geonext_shared::{territories::Territories, ServerMessage};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use warp::filters::ws::{Message, WebSocket};
use warp::Filter;

mod auth;
mod compile_utils;
#[cfg(feature = "debugging")]
mod debugging;
mod html;
mod logger;

#[macro_use]
extern crate log;

#[derive(Clone)]
pub struct State {
	/// The file system path to the client folder (passed in as command line arg)
	absolute_owned_client_path: PathBuf,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GameId(u32);

pub struct Game {
	territories: Territories,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	logger::init_logger();
	info!("Initalised logger!");

	// Extract the path from the command line args
	let absolute_owned_client_path = compile_utils::compute_path().context("Find root file path")?;

	compile_utils::compile_client(std::path::Path::new(&absolute_owned_client_path));

	#[cfg(feature = "debugging")]
	// Initalise file watcher (the watcher needs to be returned because when it is dropped the file watcher stops)
	let (_watcher, hot_reload_reciever) = debugging::initalise_filewatcher(absolute_owned_client_path.clone()).expect("Failed to initalise file watcher");

	let serve_path = absolute_owned_client_path.clone();

	let mut assets = serve_path.parent().unwrap().to_path_buf();
	assets.push("assets");
	let index_path = absolute_owned_client_path.clone();
	let index = warp::path::end().and_then(move || {
		let state = State {
			absolute_owned_client_path: index_path.clone(),
		};
		html::get_index(state)
	});
	let games = Arc::new(Mutex::new(HashMap::new()));

	match bincode::deserialize(include_bytes!("./../../assets/starting_game_map")) {
		Ok(territories) => {
			games.lock().await.insert(GameId(0), Arc::new(Mutex::new(Game { territories })));
		}
		Err(e) => error!("Failed to load map {e:?}"),
	}

	let assets = warp::path("assets").and(warp::fs::dir(assets));
	let pkg = warp::path("pkg").and(warp::fs::dir(serve_path.join("pkg")));

	let ws = warp::path("__stream").and(warp::ws()).map(move |ws: warp::ws::Ws| {
		let games = games.clone();
		let state = State {
			absolute_owned_client_path: absolute_owned_client_path.clone(),
		};

		// And then our closure will be called when it completes...
		ws.on_upgrade(|current_websocket| async move {
			let games = games.lock().await;
			let Some(game) = games.get(&GameId(0)).clone() else {
				warn!("Failed to load game");
				return;
			};
			// Just echo all messages back...
			use futures_util::stream::StreamExt;
			let (mut tx, mut rx) = current_websocket.split();
			let mut stream = Stream { stream: &mut tx };
			info!("Send map");
			if let Err(e) = stream.send(&ServerMessage::Map(game.lock().await.territories.to_rle())).await {
				error!("Failed to send map {e}")
			}
			while let Some(Ok(message)) = {
				let val = rx.next().await;
				info!("Val {val:?}");
				val
			} {
				let input = message.as_bytes();
				info!("Input {input:?}");
				let message = {
					let websocket = handle_socket_msg(&state, input, &mut stream).await.context("Handling websocket message");

					let Err(e) = websocket else { continue };
					error!("Message: {input:?}\nError: {e:?}");
					format!("{:?}", e)
				};

				let _ = stream.send(&ServerMessage::Error { message }).await;
			}
		})
	});
	let routes = index.or(ws).or(assets).or(pkg);

	#[cfg(feature = "debugging")]
	let final_routes = routes
		.or(warp::path("__reload").and_then(move || {
			let mut hot_reload_reciever = hot_reload_reciever.clone();
			async move {
				hot_reload_reciever.borrow_and_update();
				if let Err(e) = hot_reload_reciever.changed().await {
					warn!("Hot reload error {e:?}");
					return Err(warp::reject());
				}
				Ok("Plz reload")
			}
		}))
		.with(warp::cors().allow_any_origin());
	#[cfg(not(feature = "debugging"))]
	let final_routes = routes.with(warp::cors().allow_any_origin());

	warp::serve(final_routes).run(([0, 0, 0, 0, 0, 0, 0, 0], 8080)).await;

	Ok(())
}

async fn handle_socket_msg<'a>(state: &'a State, input: &'a [u8], stream: &'a mut Stream<'_>) -> anyhow::Result<()> {
	let message: geonext_shared::ClientMessage = bincode::deserialize(input).with_context(|| format!("Decode websocket binary \"{input:?}\""))?;
	let context = SocketContext { state, stream };
	match message {
		geonext_shared::ClientMessage::Auth { code } => identify(context, code).await.context("Authentication message"),
	}
}

struct Stream<'a> {
	stream: &'a mut SplitSink<WebSocket, Message>,
}
impl<'a> Stream<'a> {
	async fn send(&mut self, message: &geonext_shared::ServerMessage) -> anyhow::Result<()> {
		let response = bincode::serialize(message).expect("Serialising should sucseed");
		info!("Sending {}", format!("{message:?}").chars().take(100).collect::<String>());
		self.stream.send(Message::binary(response)).await.map_err(|e| anyhow!("Failed to send binary {e:?}"))
	}
}

struct SocketContext<'a, 'b: 'a> {
	state: &'a State,
	stream: &'a mut Stream<'b>,
}

async fn identify(context: SocketContext<'_, '_>, code: String) -> anyhow::Result<()> {
	let access_token = auth::exchange_code(&code, context.state).await.context("Exchange discord oauth code")?;
	let (_id, username) = auth::get_identity(&access_token).await.context("Get discord identity")?;
	context.stream.send(&geonext_shared::ServerMessage::AuthAccepted { username }).await?;
	Ok(())
}
