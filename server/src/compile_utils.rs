use std::path::PathBuf;

/// Finds the path to the geonext folder - can be passed as a command line argument or the executable location
pub fn compute_path() -> anyhow::Result<PathBuf> {
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

	let path = geonext_path.join("wasm-frontend");
	info!("Root file path: {}", path.to_string_lossy());
	Ok(path)
}

/// Compiles the client using `wasm-pack`, returning if successful
pub fn compile_client(path: &std::path::Path) -> bool {
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

	if !output.success() {
		error!("Wasm-pack returned with status code {}", output);
		return false;
	}

	warn!("\nServing on http://localhost:8080");

	true
}
