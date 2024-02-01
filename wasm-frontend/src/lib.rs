#![cfg(target_arch = "wasm32")]

use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::FnMut;
use std::rc::Rc;

use geonext_client::{Application, Assets, GameState};
use js_sys::{JsString, Map, Uint8Array};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

mod events;
mod html;
mod logger;
mod sockets;

#[macro_use]
extern crate log;

thread_local! {
	pub static APPLICATION_CELL: RefCell<Option<Application>> = RefCell::new(None);
	pub static HAS_CRASHED: RefCell<bool> = RefCell::new(false);
}

#[wasm_bindgen(module = "/public/utils.js")]
extern "C" {
	/// Loads assets in js (so we don't need an async runtime in rust)
	fn load_asset(asset: Map);
}

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
	logger::init().unwrap();

	std::panic::set_hook(Box::new(html::panic_hook));

	html::loading_status("assets");

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
pub fn with_assets(asset_map: Map) -> Result<(), JsValue> {
	use geonext_client::UVec2;

	let code = Some("hello world".to_string());
	info!("Got code {code:?}");
	let code = None;
	sockets::start_websocket(code).unwrap();

	html::loading_status("graphics");
	let assets = extract_assets(asset_map);

	let document = html::get_document();
	let canvas: web_sys::HtmlCanvasElement = document.get_element_by_id("canvas").unwrap().dyn_into()?;

	let window = html::get_window();
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
	APPLICATION_CELL.with(|cell| {
		if let Ok(mut old_app) = cell.try_borrow_mut() {
			old_app.replace(app);
		}
	});

	let moved_closure = Rc::new(RefCell::new(None));
	let outside_closure = moved_closure.clone();

	// Remove the loading widget
	if let Some(el) = document.get_element_by_id("loading") {
		el.set_class_name("out");
	}

	*outside_closure.borrow_mut() = Some(Closure::wrap(Box::new(move |time: f64| {
		// Update the application
		APPLICATION_CELL.with(|cell| {
			if let Ok(mut application) = cell.try_borrow_mut() {
				if let Some(application) = &mut *application {
					application.update(time as f32);
				} else {
					// Drop our handle to this closure so that it will get cleaned
					// up once we return.
					let _ = moved_closure.borrow_mut().take();
					return;
				}
			} else {
				let _ = moved_closure.borrow_mut().take();
				return;
			}
		});

		// Schedule ourself for another requestAnimationFrame callback.
		html::request_animation_frame(moved_closure.borrow().as_ref().unwrap());
	}) as Box<dyn FnMut(f64)>));

	html::request_animation_frame(outside_closure.borrow().as_ref().unwrap());

	// Bind some events
	events::add_events(window, canvas);
	Ok(())
}
