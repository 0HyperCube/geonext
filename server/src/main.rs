use anyhow::{anyhow, Context};
use futures_util::{stream::SplitSink, SinkExt};
use http_types::StatusCode;
use std::path::PathBuf;
use tokio::fs::read_to_string;
use warp::filters::ws::{Message, WebSocket};
use warp::Filter;

#[macro_use]
extern crate log;

#[cfg(feature = "debugging")]
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
#[cfg(feature = "debugging")]
use tokio::sync::watch;

// Use simplelog with a file and the console.
fn init_logger() {
	use simplelog::*;
	use std::fs::File;

	CombinedLogger::init(vec![
		TermLogger::new(LevelFilter::Info, Config::default(), TerminalMode::Mixed, ColorChoice::Auto),
		WriteLogger::new(LevelFilter::Debug, Config::default(), File::create("geonext_server.log").unwrap()),
	])
	.unwrap();

	info!("Initalised logger!");
}

#[derive(Clone)]
pub struct State {
	/// The file system path to the client folder (passed in as command line arg)
	absolute_owned_client_path: PathBuf,
}

/// Finds the path to the geonext folder - can be passed as a command line argument or the executable location
fn compute_path() -> anyhow::Result<PathBuf> {
	let geonext_path = if let Some(last_argument) = std::env::args().last() {
		let user_inputted_path = std::path::Path::new(&last_argument);
		if user_inputted_path.is_absolute() {
			user_inputted_path.to_owned()
		} else {
			let current_dir = std::env::current_dir()?;
			current_dir.join(user_inputted_path)
		}
	} else {
		std::env::current_exe()?
	};
	for ancestor in geonext_path.ancestors() {
		if ancestor.ends_with("geonext") {
			return Ok(ancestor.to_path_buf().join("wasm-frontend"));
		}
	}
	Ok(geonext_path.join("wasm-frontend"))
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	init_logger();

	// Extract the path from the command line args
	let absolute_owned_client_path = compute_path().context("Find root file path")?;

	info!("Root file path: {}", absolute_owned_client_path.to_string_lossy());

	compile_client(std::path::Path::new(&absolute_owned_client_path));

	#[cfg(feature = "debugging")]
	// Initalise file watcher (the watcher needs to be returned because when it is dropped the file watcher stops)
	let (_watcher, hot_reload_reciever) = initalise_filewatcher(absolute_owned_client_path.clone()).expect("Failed to initalise file watcher");

	let serve_path = absolute_owned_client_path.clone();

	let mut assets = serve_path.parent().unwrap().to_path_buf();
	assets.push("assets");
	let index_path = absolute_owned_client_path.clone();
	let index = warp::path::end().and_then(move || {
		let state = State {
			absolute_owned_client_path: index_path.clone(),
		};
		get_index(state)
	});
	let pkg = warp::path("assets").and(warp::path("pkg")).and(warp::fs::dir(assets.join("pkg")));
	let public = warp::path("assets").and(warp::fs::dir(assets.join("public")));

	let ws = warp::path("__stream").and(warp::ws()).map(move |ws: warp::ws::Ws| {
		let state = State {
			absolute_owned_client_path: absolute_owned_client_path.clone(),
		};
		// And then our closure will be called when it completes...
		ws.on_upgrade(|current_websocket| async move {
			// Just echo all messages back...
			use futures_util::stream::StreamExt;
			let (mut tx, mut rx) = current_websocket.split();
			while let Some(Ok(message)) = rx.next().await {
				let Ok(input) = message.to_str() else {
					error!("Recieved message of bad type");
					continue;
				};
				let mut stream = Stream { stream: &mut tx };
				let message = {
					let websocket = handle_socket_msg(&state, input, &mut stream).await.context("Handling websocket message");

					let Err(e) = websocket else { continue };
					error!("Message: {input}\nError: {e:?}");
					format!("{e:?}")
				};

				let _ = stream.send(&geonext_shared::ServerMessage::Error { message }).await;
			}
		})
	});
	let routes = index.or(ws).or(public).or(pkg).or(warp::fs::dir(serve_path));

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

async fn handle_socket_msg<'a>(state: &'a State, input: &'a str, stream: &'a mut Stream<'_>) -> anyhow::Result<()> {
	let serde_json: geonext_shared::ClientMessage = serde_json::from_str(input).with_context(|| format!("Decode websocket json \"{input}\""))?;
	let context = SocketContext { state, input, stream };
	match serde_json {
		geonext_shared::ClientMessage::Auth { code } => identify(context, code).await.context("Authentication message"),
	}
}

struct Stream<'a> {
	stream: &'a mut SplitSink<WebSocket, Message>,
}
impl<'a> Stream<'a> {
	async fn send(&mut self, message: &geonext_shared::ServerMessage) -> anyhow::Result<()> {
		let response = serde_json::to_string(message).expect("Serialising should sucseed");
		self.stream.send(Message::text(response)).await.map_err(|e| anyhow!("Failed to send json {e:?}"))
	}
}

struct SocketContext<'a, 'b: 'a> {
	state: &'a State,
	input: &'a str,
	stream: &'a mut Stream<'b>,
}

async fn identify(context: SocketContext<'_, '_>, code: String) -> anyhow::Result<()> {
	let access_token = exchange_code(&code, context.state).await.context("Exchange discord oauth code")?;
	let (_id, username) = get_identity(&access_token).await.context("Get discord identity")?;
	context.stream.send(&geonext_shared::ServerMessage::AuthAccepted { username }).await?;
	Ok(())
}

#[cfg(feature = "debugging")]
fn add_hot_reload_javascript(mut body: String) -> String {
	// Remove the closing tags from the document (to allow js to be inserted in firefox)
	for closing_tag in ["</body>", "</html>"] {
		if let Some(pos) = body.find(closing_tag) {
			for _ in 0..closing_tag.len() {
				body.remove(pos);
			}
		}
	}

	// Insert some hotreload js
	body += r#"<!--Inserted hotreload script--> <script type="module">try {
			console.info("initalised hotreaload");
			await fetch("__reload")
		} catch (e){
			console.warn("Failed to wait for hotreload.");
		}
		window.location.reload(true);
		</script>"#;
	body
}

async fn read_file(file_name: &'static str, state: &State) -> anyhow::Result<String> {
	let mut owned_client_path = state.absolute_owned_client_path.clone();
	owned_client_path.push(file_name);
	Ok(read_to_string(&owned_client_path).await.with_context(|| format!("reading file {owned_client_path:?}"))?)
}

async fn insert_standard_head(mut body: String, state: &State) -> anyhow::Result<String> {
	let insert_below = r#"<html lang="en">"#;
	if let Some(pos) = body.find(insert_below) {
		body.insert_str(pos + insert_below.len(), &read_file("standard_head.html", state).await.context("Insert standard head")?)
	}
	Ok(body)
}

async fn generate_index(state: &State) -> anyhow::Result<String> {
	insert_standard_head(read_file("index.html", &state).await?, &state).await
}

/// Returns the index.html file, inserting a hot reload script if debug is enabled
async fn get_index(state: State) -> Result<warp::reply::Html<String>, warp::Rejection> {
	let index = match generate_index(&state).await {
		Ok(index) => index,
		Err(e) => {
			error!("Error {e:?}");
			return Err(warp::reject());
		}
	};

	#[cfg(feature = "debugging")]
	{
		Ok(warp::reply::html(add_hot_reload_javascript(index)))
	}
	#[cfg(not(feature = "debugging"))]
	{
		Ok(warp::reply::html(index))
	}

	// if let Some(code) = param.get("code") {
	// 	info!("Logged in with code {code}");
	// } else {
	// 	// let welcome = insert_standard_head(read_file("welcome.html", &state).await.unwrap(), &state).await.unwrap();

	// 	// Ok(warp::reply::html(welcome))
	// }
}

const API_ENDPOINT: &str = "https://discord.com/api/v10";
const CLIENT_ID: &str = "1072924944050159722";
const GUILD_ID: &str = "891386654714122282";

async fn exchange_code(code: &str, state: &State) -> anyhow::Result<String> {
	let client_secret = read_file("client_secret.txt", state).await.context("Getting client secret file")?;

	let body = form_urlencoded::Serializer::new(String::new())
		.append_pair("client_id", CLIENT_ID)
		.append_pair("client_secret", client_secret.trim())
		.append_pair("grant_type", "authorization_code")
		.append_pair("code", code)
		.append_pair("redirect_uri", "http://127.0.0.1:8080")
		.finish();

	let url = &format!("{API_ENDPOINT}/oauth2/token");
	let response = surf::post(url)
		.body(body)
		.content_type("application/x-www-form-urlencoded")
		.recv_string()
		.await
		.map_err(|e| anyhow!("Requesting exchange code {e:?}"))?;

	let response_json: serde_json::Value = serde_json::from_str(&response).context("Deserialising exchange code")?;
	let access_token = &response_json["access_token"];

	access_token.as_str().map(|x| x.to_string()).ok_or(anyhow!("No access token: {response}"))
}

async fn get_identity(access_token: &str) -> anyhow::Result<(String, String)> {
	let url = format!("{API_ENDPOINT}/users/@me");
	let response = surf::get(url)
		//.body(body)
		//.content_type("application/json")
		.header("Authorization", format!("Bearer {access_token}"))
		.header("User-Agent", "GeoNext (http://127.0.0.1:8080, 1)")
		.recv_string()
		.await
		.map_err(|e| anyhow!("get_identity {e:?}"))?;

	let response_json: serde_json::Value = serde_json::from_str(&response).context("Deserialising identity")?;
	info!("Response {response}");
	let id = &response_json["id"];
	let username = &response_json["username"];

	let id = id.as_str().map(|x| x.to_string()).ok_or(anyhow!("No id: {response}"))?;
	let username = username.as_str().map(|x| x.to_string()).ok_or(anyhow!("No username: {response}"))?;

	let response = surf::get(format!("{API_ENDPOINT}/guilds/{GUILD_ID}/members/{id}"))
		.header("Authorization", format!("Bearer {access_token}"))
		.header("User-Agent", "GeoNext (http://127.0.0.1:8080, 1)")
		.recv_string()
		.await
		.map_err(|e| anyhow!("get_identity {e:?}"))?;
	let response_json: serde_json::Value = serde_json::from_str(&response).context("Deserialising identity")?;
	let username = response_json.get("nick").and_then(|f| f.as_str().map(|x| x.to_string())).unwrap_or(username);

	Ok((id, username))
}

#[cfg(feature = "debugging")]
fn process_file_watcher_event(result: Result<Event, notify::Error>, absolute_owned_client_path: &PathBuf, hot_reload_sender: &watch::Sender<()>) {
	let event = result.unwrap();

	// Skip non-modify events or events in ignored files like `.lock` `target/` or `/pkg/`
	let is_reload = event.kind.is_modify()
		&& event.paths.iter().any(|path| {
			let string = path.to_string_lossy();
			!string.contains("target/") && !string.contains(".lock") && !string.contains("/pkg/") && !string.contains(".git") && !string.contains(".log")
		});
	if is_reload {
		// Clear screen
		print!("\x1B[2J\x1B[1;1H");

		info!("Live refresh of: {}", event.paths.iter().filter_map(|path| path.to_str()).collect::<Vec<_>>().join(", "));

		if compile_client(std::path::Path::new(&absolute_owned_client_path)) {
			let _ = hot_reload_sender.send(());
		}
	}
}

#[cfg(feature = "debugging")]
/// Initalise file watcher (the watcher needs to be returned because when it is dropped the file watcher stops)
fn initalise_filewatcher(absolute_owned_client_path: PathBuf) -> notify::Result<(notify::RecommendedWatcher, watch::Receiver<()>)> {
	// Watch the parent of the client directory
	let watch = absolute_owned_client_path.parent().unwrap();
	let absolute_owned_client_path = absolute_owned_client_path.clone();

	// Create channel for sending reload events to clients
	let (hot_reload_sender, hot_reload_reciever) = watch::channel(());
	let event_handler = move |result: Result<Event, notify::Error>| process_file_watcher_event(result, &absolute_owned_client_path, &hot_reload_sender);
	let mut watcher = RecommendedWatcher::new(event_handler, notify::Config::default())?;

	watcher.watch(watch, RecursiveMode::Recursive)?;
	info!("Watching directory {:?}", watch);
	Ok((watcher, hot_reload_reciever))
}

/// Compiles the client using `wasm-pack`, returning if successful
fn compile_client(path: &std::path::Path) -> bool {
	use std::process::Command;

	println!("\n{}\n\nCompiling client at {}\n\n", "=".repeat(100), path.to_str().unwrap_or_default());

	// Choose optimisation level based on the debugging feature
	#[cfg(feature = "debugging")]
	let optimisations = "--debug";
	#[cfg(not(feature = "debugging"))]
	let optimisations = "--release";

	// Execute wasm-pack in a child process
	let mut cmd = Command::new("wasm-pack");
	let output = match cmd
		.args(&["build", "--target", "web", "--no-typescript", optimisations, "--", "--color", "always"])
		.current_dir(path)
		.status()
	{
		Ok(x) => x,
		Err(e) => {
			error!("Failed to run wasm-pack: {e:?}");
			return false;
		}
	};

	// Show the user the output from wasm-pack
	// io::stdout().write_all(&output.stdout).unwrap();
	// io::stderr().write_all(&output.stderr).unwrap();

	// let out = output.wait().expect("Could not wait");
	// while out.code() == Some(2) {}
	if !output.success() {
		error!("Wasm-pack returned with status code {}", output);
		return false;
	}

	warn!("\nServing on http://localhost:8080");

	true
}
