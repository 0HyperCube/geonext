use geonext_shared::ServerMessage;
use wasm_bindgen::{closure::Closure, JsCast, JsValue};

pub fn start_websocket(code: Option<String>) -> Result<(), JsValue> {
	let location = web_sys::window().unwrap().location().host()?;
	let ws = web_sys::WebSocket::new(&format!("ws://{location}/__stream"))?;
	ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

	// create callback
	let _cloned_ws = ws.clone();
	let onmessage_callback = Closure::<dyn FnMut(_)>::new(move |e: web_sys::MessageEvent| {
		if let Ok(data) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
			let value = js_sys::Uint8Array::new(&data).to_vec();

			let Ok(message) = bincode::deserialize::<geonext_shared::ServerMessage>(&value) else {
				error!("Recieved malformed message.");
				return;
			};
			crate::APPLICATION_CELL.with(|cell| {
				if let Ok(mut application) = cell.try_borrow_mut() {
					if let Some(application) = &mut *application {
						application.event(geonext_client::EventType::Message(message));
					}
				}
			});
		} else {
			warn!("message event, received Unknown: {:?}", e.data());
		}
	});
	// set message event handler on WebSocket
	ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
	// forget the callback to keep it alive
	onmessage_callback.forget();

	// Log any errors
	let onerror_callback = Closure::<dyn FnMut(_)>::new(move |e: web_sys::ErrorEvent| {
		warn!("Error event");
		warn!("error event: {:?}", e.type_());
		warn!("error event: {:?}", js_sys::JSON::stringify(&e));
		warn!("Erro")
	});
	ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
	onerror_callback.forget();

	// Check on socket open
	let cloned_ws = ws.clone();
	let onopen_callback = Closure::<dyn FnMut()>::new(move || {
		info!("socket opened");

		if let Some(code) = code.clone() {
			let data = bincode::serialize(&geonext_shared::ClientMessage::Auth { code }).unwrap();
			let buffer = js_sys::Uint8Array::new_with_length(data.len() as u32);
			buffer.copy_from(&data);
			let buffer = buffer.buffer();
			match cloned_ws.send_with_array_buffer(&buffer) {
				Ok(_) => info!("message successfully sent"),
				Err(err) => error!("error sending message: {:?}", err),
			}
		}
	});
	ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
	onopen_callback.forget();

	Ok(())
}
