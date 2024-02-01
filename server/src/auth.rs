use anyhow::{anyhow, Context};

use crate::{html::read_file, State};

const API_ENDPOINT: &str = "https://discord.com/api/v10";
const CLIENT_ID: &str = "1072924944050159722";
const GUILD_ID: &str = "891386654714122282";

pub async fn exchange_code(code: &str, state: &State) -> anyhow::Result<String> {
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

pub async fn get_identity(access_token: &str) -> anyhow::Result<(String, String)> {
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
