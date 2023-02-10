#![cfg(target_arch = "wasm32")]

use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::FnMut;
use std::ops::FnOnce;
use std::rc::Rc;

use geonext_client::{Application, Assets, GameState};
use js_sys::{JsString, Map, Uint8Array};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

mod logger;

#[macro_use]
extern crate log;

thread_local! {
	pub static APPLICATION_CELL: RefCell<Option<Application>> = RefCell::new(None);
	pub static HAS_CRASHED: RefCell<bool> = RefCell::new(false);
}

#[wasm_bindgen(module = "/utils.js")]
extern "C" {
	/// Loads assets in js (so we don't need an async runtime in rust)
	fn load_asset(asset: Map);
}

/// Gets the window object with websys
fn get_window() -> web_sys::Window {
	web_sys::window().expect("no global `window` exists")
}

/// Gets the document object with websys
fn get_document() -> web_sys::Document {
	get_window().document().expect("should have a document on window")
}

/// Requests an animation frame from websys. This has to be called every frame if you want another frame.
fn request_animation_frame(f: &Closure<dyn FnMut(f64)>) {
	get_window().request_animation_frame(f.as_ref().unchecked_ref()).expect("Failed to register `requestAnimationFrame`");
}

/// Adds an event listner to a specific target with a closure with no arguments
fn add_event_listener(target: &web_sys::EventTarget, event: &str, f: Closure<dyn FnMut()>) {
	target
		.add_event_listener_with_callback(event, f.as_ref().unchecked_ref())
		.expect(&format!("Failed to register `{event}`"));
	// Leak memory :)
	f.forget();
}

/// Adds an event listner on the document that sends an event
fn add_document_event<T, F>(event_name: &'static str, callback: F)
where
	T: wasm_bindgen::convert::FromWasmAbi + 'static,
	F: Fn(&mut Application, T) -> geonext_client::EventType + 'static,
{
	let closure = Closure::wrap(Box::new(move |e: T| {
		APPLICATION_CELL.with(|x| {
			if let Some(application) = &mut *x.borrow_mut() {
				let event = callback(application, e);
				application.event(event);
			}
		});
	}) as Box<dyn FnMut(T)>);
	get_document()
		.add_event_listener_with_callback(event_name, closure.as_ref().unchecked_ref())
		.expect(&format!("should register `{event_name}` OK"));
	// Leak memory :)
	closure.forget();
}

/// Sets the loading status which is displayed to the user
fn loading_status(status: &str) {
	if let Some(el) = get_document().get_element_by_id("loadingcomponent") {
		el.set_inner_html(status);
	}
}

/// When a panic occurs, notify the user and log the error to the JS console
pub fn panic_hook(info: &core::panic::PanicInfo) {
	// Skip if we have already panicked
	if HAS_CRASHED.with(|cell| cell.replace(true)) {
		return;
	}
	error!("{}", info);

	let document = get_document();
	if let Some(el) = document.get_element_by_id("errorreason") {
		el.set_inner_html(&format!("{info}"));
	}
}

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
	logger::init().unwrap();

	std::panic::set_hook(Box::new(panic_hook));

	loading_status("assets");

	// Collect assets into a js map object
	let map = Map::new();
	for (key, value) in Assets::assets().into_iter() {
		map.set(&JsString::from(*key), &JsString::from(*value));
	}

	load_asset(map);

	Ok(())
}

/// Extract the loaded assets from a js map object
fn extract_assets(asset_map: Map) -> Assets {
	let mut assets = HashMap::new();
	asset_map.for_each(&mut |value, key| {
		let buf = value.dyn_into::<Uint8Array>().unwrap();
		let mut dest = vec![0; buf.length() as usize];
		buf.copy_to(&mut dest);
		assets.insert(key.as_string().unwrap(), dest);
	});
	Assets(assets)
}

/// Creates the webgl context from the html canvas element, returning the [`glow::Context`] and the width and height as `u32`s.
fn create_gl_context(window: &web_sys::Window, canvas: &web_sys::HtmlCanvasElement) -> Result<(glow::Context, u32, u32), JsValue> {
	let width = window.inner_width()?.as_f64().unwrap() as u32;
	let height = window.inner_height()?.as_f64().unwrap() as u32;
	canvas.set_width(width);
	canvas.set_height(height);

	let mut attrs = web_sys::WebGlContextAttributes::new();
	attrs.stencil(true);
	attrs.antialias(true);
	let webgl1_context = match canvas.get_context_with_context_options("webgl2", attrs.as_ref()) {
		Ok(Some(context)) => context.dyn_into::<web_sys::WebGl2RenderingContext>().unwrap(),
		_ => panic!("Canvas::getContext failed to retrieve WebGL 2 context"),
	};

	Ok((glow::Context::from_webgl2_context(webgl1_context), width, height))
}

#[wasm_bindgen]
#[cfg(target_arch = "wasm32")]
pub fn with_assets(asset_map: Map, code: Option<String>) -> Result<(), JsValue> {
	use geonext_client::{EventType, UVec2};

	info!("Got code {code:?}");
	start_websocket(code);

	loading_status("graphics");
	let assets = extract_assets(asset_map);

	let document = get_document();
	let canvas: web_sys::HtmlCanvasElement = document.get_element_by_id("canvas").unwrap().dyn_into()?;

	let window = get_window();
	let (context, width, height) = create_gl_context(&window, &canvas)?;

	// Construct our application

	let game_state = GameState {
		viewport: UVec2::new(width, height),
		scale_factor: 1.,
		..Default::default()
	};
	let app = match Application::new(game_state, context, assets) {
		Ok(app) => app,
		Err(e) => {
			panic!("{}", &e);
		}
	};
	APPLICATION_CELL.with(|cell| cell.borrow_mut().replace(app));

	let moved_closure = Rc::new(RefCell::new(None));
	let outside_closure = moved_closure.clone();

	// Remove the loading widget
	if let Some(el) = document.get_element_by_id("loading") {
		el.set_class_name("out");
	}

	let textelement = document.get_element_by_id("t").unwrap();
	*outside_closure.borrow_mut() = Some(Closure::wrap(Box::new(move |time: f64| {
		// Drop our handle to this closure so that it will get cleaned
		// up once we return.
		// let _ = moved_closure.borrow_mut().take();
		// return;

		// Update the application
		APPLICATION_CELL.with(|cell| {
			if let Some(application) = &mut *cell.borrow_mut() {
				application.update(time as f32);
				let text = format!("Peak: {}ms", application.game_state.time.peak_frametime().round());
				textelement.set_text_content(Some(&text));
			} else {
				panic!("No app");
			}
		});

		// Schedule ourself for another requestAnimationFrame callback.
		request_animation_frame(moved_closure.borrow().as_ref().unwrap());
	}) as Box<dyn FnMut(f64)>));

	request_animation_frame(outside_closure.borrow().as_ref().unwrap());

	let closure = Closure::wrap(Box::new(move || {
		let window = get_window();
		let width = window.inner_width().unwrap().as_f64().unwrap() as u32;
		let height = window.inner_height().unwrap().as_f64().unwrap() as u32;
		canvas.set_width(width);
		canvas.set_height(height);
		APPLICATION_CELL.with(|cell| {
			if let Some(application) = &mut *cell.borrow_mut() {
				application.resize(width, height);
			}
		});
	}) as Box<dyn FnMut()>);

	// Bind some events

	add_event_listener(&window, "resize", closure);

	fn update_pointer_position(application: &mut Application, e: &web_sys::PointerEvent) -> geonext_client::IVec2 {
		application.game_state.input.using_touch = e.pointer_type() == "touch";
		if application.game_state.input.using_touch {
			panic!("Touch is not yet supported. Please implement it yourself");
		}
		application.game_state.input.update_mouse(e.x(), e.y(), e.buttons())
	}
	add_document_event("pointerdown", |application, e| {
		update_pointer_position(application, &e);
		EventType::PointerDown(geonext_client::MouseButton::from_num(e.button()))
	});
	add_document_event("pointerup", |application, e| {
		update_pointer_position(application, &e);
		EventType::PointerUp(geonext_client::MouseButton::from_num(e.button()))
	});
	add_document_event("pointermove", |application, e| EventType::PointerMove(update_pointer_position(application, &e)));

	let wheel_callback = |_: &mut Application, e: web_sys::WheelEvent| EventType::PointerScroll((e.delta_x() as f32, e.delta_y() as f32).into());
	add_document_event("wheel", wheel_callback);

	let key_down = |_: &mut Application, e: web_sys::KeyboardEvent| EventType::KeyDown(e.key());
	add_document_event("keydown", key_down);

	let key_up = |_: &mut Application, e: web_sys::KeyboardEvent| EventType::KeyUp(e.key());
	add_document_event("keyup", key_up);

	Ok(())
}

fn start_websocket(code: Option<String>) -> Result<(), JsValue> {
	let ws = web_sys::WebSocket::new("ws://localhost:8080/__stream")?;
	// create callback
	let cloned_ws = ws.clone();
	let onmessage_callback = Closure::<dyn FnMut(_)>::new(move |e: web_sys::MessageEvent| {
		if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
			info!("message event, received Text: {:?}", txt);
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
		error!("error event: {:?}", e);
	});
	ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
	onerror_callback.forget();

	// Check on socket open
	let cloned_ws = ws.clone();
	let onopen_callback = Closure::<dyn FnMut()>::new(move || {
		if let Some(code) = code.clone() {
			info!("socket opened");
			let data = serde_json::to_string(&geonext_shared::ClientMessage::Auth { code }).unwrap();
			match cloned_ws.send_with_str(&data) {
				Ok(_) => info!("message successfully sent"),
				Err(err) => error!("error sending message: {:?}", err),
			}
		}
	});
	ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
	onopen_callback.forget();

	Ok(())
}
