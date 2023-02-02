use std::path::PathBuf;

use async_std::fs::read_to_string;
use tide::{Request, Response};

#[cfg(feature = "debugging")]
use async_std::channel::*;
#[cfg(feature = "debugging")]
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};

#[derive(Clone)]
struct State {
	#[cfg(feature = "debugging")]
	/// Sends an empty value whenever the client is rebuilt (in debugging mode)
	hot_reload_reciever: Receiver<()>,
	/// The file system path to the client folder (passed in as command line arg)
	absolute_owned_client_path: PathBuf,
}

fn compute_path() -> PathBuf {
	let geonext_path = if let Some(last_argument) = std::env::args().last() {
		let user_inputted_path = std::path::Path::new(&last_argument);
		if user_inputted_path.is_absolute() {
			user_inputted_path.to_owned()
		} else {
			let current_dir = std::env::current_dir().unwrap();
			current_dir.join(user_inputted_path)
		}
	} else {
		std::env::current_exe().unwrap()
	};
	for ancestor in geonext_path.ancestors() {
		if ancestor.ends_with("geonext") {
			return ancestor.to_path_buf().join("wasm-frontend");
		}
	}
	geonext_path.join("wasm-frontend")
}

// Because we're running a web server we need a runtime,
// for more information on async runtimes, please check out [async-std](https://github.com/async-rs/async-std)
#[async_std::main]
async fn main() -> tide::Result<()> {
	// Extract the path from the command line args
	let absolute_owned_client_path = compute_path();

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
	let mut assets = serve_path.parent().unwrap().to_path_buf();
	assets.push("assets");
	app.at("/assets").serve_dir(&assets).unwrap();

	app.at("/").serve_dir(serve_path).unwrap();
	app.listen("127.0.0.1:8080").await?;

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

/// Returns the index.html file, inserting a hot reload script if debug is enabled
async fn get_index(req: Request<State>) -> tide::Result {
	let mut owned_client_path = req.state().absolute_owned_client_path.clone();
	owned_client_path.push("index.html");
	let path = std::path::Path::new(&owned_client_path);

	let mut res = Response::new(200);
	res.set_content_type("text/html;charset=utf-8");

	#[cfg(feature = "debugging")]
	res.set_body(add_hot_reload_javascript(read_to_string(path).await.unwrap()));

	#[cfg(not(feature = "debugging"))]
	res.set_body(read_to_string(path).await.unwrap());

	Ok(res)
}

#[cfg(feature = "debugging")]
fn process_file_watcher_event(result: Result<Event, notify::Error>, absolute_owned_client_path: &PathBuf, hot_reload_sender: &Sender<()>) {
	let event = result.unwrap();

	// Skip non-modify events or events in ignored files like `.lock` `target/` or `/pkg/`
	let is_reload = event.kind.is_modify()
		&& event.paths.iter().any(|path| {
			let string = path.to_string_lossy();
			!string.contains("target/") && !string.contains(".lock") && !string.contains("/pkg/") && !string.contains(".git")
		});
	if is_reload {
		// Clear screen
		print!("\x1B[2J\x1B[1;1H");

		println!("Live refresh of: {}", event.paths.iter().filter_map(|path| path.to_str()).collect::<Vec<_>>().join(", "));

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
	println!("Watching directory {:?}", watch);
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
		.stderr(Stdio::piped())
		.stdout(Stdio::piped())
		.output()
	{
		Ok(x) => x,
		_ => return false,
	};

	// Show the user the output from wasm-pack
	io::stdout().write_all(&output.stdout).unwrap();
	io::stderr().write_all(&output.stderr).unwrap();

	// let out = output.wait().expect("Could not wait");
	// while out.code() == Some(2) {}
	if !output.status.success() {
		return false;
	}

	println!("\nServing on http://127.0.0.1:8080");

	true
}
