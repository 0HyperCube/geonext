use std::path::PathBuf;

use anyhow::{anyhow, Context};
use async_std::fs::read_to_string;
use tide::{Request, Response};

#[macro_use]
extern crate log;

#[cfg(feature = "debugging")]
use async_std::channel::*;
#[cfg(feature = "debugging")]
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use tide_websockets::WebSocketConnection;

// Use simplelog with a file and the console.
fn init_logger() {
	use simplelog::*;
	use std::fs::File;

	CombinedLogger::init(vec![
		TermLogger::new(LevelFilter::Info, Config::default(), TerminalMode::Mixed, ColorChoice::Auto),
		WriteLogger::new(LevelFilter::Debug, Config::default(), File::create("CheeseBot.log").unwrap()),
	])
	.unwrap();

	info!("Initalised logger!");
}

#[derive(Clone)]
pub struct State {
	#[cfg(feature = "debugging")]
	/// Sends an empty value whenever the client is rebuilt (in debugging mode)
	hot_reload_reciever: Receiver<()>,
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

// Because we're running a web server we need a runtime,
// for more information on async runtimes, please check out [async-std](https://github.com/async-rs/async-std)
#[async_std::main]
async fn main() -> tide::Result<()> {
	init_logger();

	// Extract the path from the command line args
	let absolute_owned_client_path = compute_path().context("Find root file path")?;

	info!("Root file path: {}", absolute_owned_client_path.to_string_lossy());

	compile_client(std::path::Path::new(&absolute_owned_client_path));

	#[cfg(feature = "debugging")]
	// Initalise file watcher (the watcher needs to be returned because when it is dropped the file watcher stops)
	let (_watcher, hot_reload_reciever) = initalise_filewatcher(absolute_owned_client_path.clone()).expect("Failed to initalise file watcher");

	let serve_path = absolute_owned_client_path.clone();

	// We set up a web server using [Tide](https://github.com/http-rs/tide)
	let mut app = tide::with_state(State {
		#[cfg(feature = "debugging")]
		hot_reload_reciever,
		absolute_owned_client_path,
	});

	app.at("/").get(get_index);

	#[cfg(feature = "debugging")]
	app.at("/__reload").get(get_reload);

	app.at("/__auth").get(get_index);

	app.at("/__stream").get(tide_websockets::WebSocket::new(|request, mut stream| async move {
		use async_std::stream::StreamExt;
		while let Some(Ok(tide_websockets::Message::Text(input))) = stream.next().await {
			let stream = Stream { stream: &mut stream };
			if let Err(e) = websocket(&request, &input, &stream).await.context("Handling websocket message") {
				error!("Message: {input}\nError: {e:?}");
				stream.send(&geonext_shared::ServerMessage::Error { message: format!("{e:?}") }).await?;
			}
		}

		Ok(())
	}));

	let mut assets = serve_path.parent().unwrap().to_path_buf();
	assets.push("assets");
	app.at("/assets").serve_dir(&assets).unwrap();

	app.at("/").serve_dir(serve_path).unwrap();
	app.listen("127.0.0.1:8080").await?;

	Ok(())
}

async fn websocket(request: &Request<State>, input: &str, stream: &Stream<'_>) -> anyhow::Result<()> {
	let serde_json: geonext_shared::ClientMessage = serde_json::from_str(input).with_context(|| format!("Decode websocket json \"{input}\""))?;
	let context = SocketContext { request, input, stream };
	match serde_json {
		geonext_shared::ClientMessage::Auth { code } => identify(context, code).await.context("Authentication message"),
	}
}

struct Stream<'a> {
	stream: &'a mut WebSocketConnection,
}
impl<'a> Stream<'a> {
	async fn send(&self, message: &geonext_shared::ServerMessage) -> anyhow::Result<()> {
		let response = serde_json::to_string(message).expect("Serialising should sucseed");
		self.stream.send_string(response).await.map_err(|e| anyhow!("Failed to send json {e:?}"))
	}
}

struct SocketContext<'a> {
	request: &'a Request<State>,
	input: &'a str,
	stream: &'a Stream<'a>,
}

async fn identify(context: SocketContext<'_>, code: String) -> anyhow::Result<()> {
	let access_token = exchange_code(&code, context.request).await.context("Exchange discord oauth code")?;
	let (id, username) = get_identity(&access_token).await.context("Get discord identity")?;
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
			console.info("hotreloading")
			window.location.reload(true);
		} catch (e){
			console.warn("Failed to wait for hotreload.");
		}</script>"#;
	body
}

async fn read_file(file_name: &'static str, req: &Request<State>) -> anyhow::Result<String> {
	let mut owned_client_path = req.state().absolute_owned_client_path.clone();
	owned_client_path.push(file_name);
	Ok(read_to_string(&owned_client_path).await.context("reading file")?)
}

async fn insert_standard_head(mut body: String, req: &Request<State>) -> anyhow::Result<String> {
	let insert_below = r#"<html lang="en">"#;
	if let Some(pos) = body.find(insert_below) {
		body.insert_str(pos + insert_below.len(), &read_file("standard_head.html", req).await.context("Insert standard head")?)
	}
	Ok(body)
}

/// Returns the index.html file, inserting a hot reload script if debug is enabled
async fn get_index(req: Request<State>) -> tide::Result {
	if !req.url().query_pairs().any(|(key, _)| key == "code") {
		let index = insert_standard_head(read_file("index.html", &req).await?, &req).await?;

		let mut res = Response::new(200);
		res.set_content_type("text/html;charset=utf-8");

		#[cfg(feature = "debugging")]
		res.set_body(add_hot_reload_javascript(index));

		#[cfg(not(feature = "debugging"))]
		res.set_body(index);

		Ok(res)
	} else {
		let welcome = insert_standard_head(read_file("welcome.html", &req).await?, &req).await?;

		let mut res = Response::new(200);
		res.set_content_type("text/html;charset=utf-8");

		res.set_body(welcome);

		Ok(res)
	}
}

const API_ENDPOINT: &str = "https://discord.com/api/v10";
const CLIENT_ID: &str = "1072924944050159722";

async fn exchange_code(code: &str, req: &Request<State>) -> anyhow::Result<String> {
	let client_secret = read_file("client_secret.txt", req).await.context("Getting client secret file")?;

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

	Ok((
		id.as_str().map(|x| x.to_string()).ok_or(anyhow!("No id: {response}"))?,
		username.as_str().map(|x| x.to_string()).ok_or(anyhow!("No username: {response}"))?,
	))
}

#[cfg(feature = "debugging")]
fn process_file_watcher_event(result: Result<Event, notify::Error>, absolute_owned_client_path: &PathBuf, hot_reload_sender: &Sender<()>) {
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
			let _ = hot_reload_sender.send_blocking(());
		}
	}
}

#[cfg(feature = "debugging")]
/// Initalise file watcher (the watcher needs to be returned because when it is dropped the file watcher stops)
fn initalise_filewatcher(absolute_owned_client_path: PathBuf) -> notify::Result<(notify::RecommendedWatcher, Receiver<()>)> {
	// Watch the parent of the client directory
	let watch = absolute_owned_client_path.parent().unwrap();
	let absolute_owned_client_path = absolute_owned_client_path.clone();

	// Create channel for sending reload events to clients
	let (hot_reload_sender, hot_reload_reciever) = bounded(1);
	let event_handler = move |result: Result<Event, notify::Error>| process_file_watcher_event(result, &absolute_owned_client_path, &hot_reload_sender);
	let mut watcher = RecommendedWatcher::new(event_handler, notify::Config::default())?;

	watcher.watch(watch, RecursiveMode::Recursive)?;
	info!("Watching directory {:?}", watch);
	Ok((watcher, hot_reload_reciever))
}

/// Responds to a request when the client is recompiled
#[cfg(feature = "debugging")]
async fn get_reload(req: Request<State>) -> tide::Result {
	req.state().hot_reload_reciever.recv().await.unwrap();

	let mut res = Response::new(200);
	res.set_content_type("text/html;charset=utf-8");
	let body = "Plz reload";

	res.set_body(body);
	Ok(res)
}

/// Compiles the client using `wasm-pack`, returning if successful
fn compile_client(path: &std::path::Path) -> bool {
	use std::io::{self, Write};
	use std::process::{Command, Stdio};

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

	warn!("\nServing on http://127.0.0.1:8080");

	true
}
