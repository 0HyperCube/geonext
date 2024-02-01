use crate::State;
use anyhow::Context;
use tokio::fs::read_to_string;

pub async fn read_file(file_name: &'static str, state: &State) -> anyhow::Result<String> {
	let mut owned_client_path = state.absolute_owned_client_path.clone();
	owned_client_path.push(file_name);
	Ok(read_to_string(&owned_client_path).await.with_context(|| format!("reading file {owned_client_path:?}"))?)
}

pub async fn insert_standard_head(mut body: String, state: &State) -> anyhow::Result<String> {
	let insert_below = r#"<html lang="en">"#;
	if let Some(pos) = body.find(insert_below) {
		body.insert_str(pos + insert_below.len(), &read_file("standard_head.html", state).await.context("Insert standard head")?)
	}
	Ok(body)
}

pub async fn generate_index(state: &State) -> anyhow::Result<String> {
	insert_standard_head(read_file("index.html", &state).await?, &state).await
}

/// Returns the index.html file, inserting a hot reload script if debug is enabled
pub async fn get_index(state: State) -> Result<warp::reply::Html<String>, warp::Rejection> {
	let index = match generate_index(&state).await {
		Ok(index) => index,
		Err(e) => {
			error!("Error {e:?}");
			return Err(warp::reject());
		}
	};

	#[cfg(feature = "debugging")]
	{
		Ok(warp::reply::html(crate::debugging::add_hot_reload_javascript(index)))
	}
	#[cfg(not(feature = "debugging"))]
	{
		Ok(warp::reply::html(index))
	}
}
