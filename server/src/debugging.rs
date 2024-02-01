use crate::compile_utils::compile_client;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use tokio::sync::watch;

pub fn process_file_watcher_event(result: Result<Event, notify::Error>, absolute_owned_client_path: &PathBuf, hot_reload_sender: &watch::Sender<()>) {
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

/// Initalise file watcher (the watcher needs to be returned because when it is dropped the file watcher stops)
pub fn initalise_filewatcher(absolute_owned_client_path: PathBuf) -> notify::Result<(notify::RecommendedWatcher, watch::Receiver<()>)> {
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

pub fn add_hot_reload_javascript(mut body: String) -> String {
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
