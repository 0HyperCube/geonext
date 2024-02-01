use std::ops::FnMut;

use geonext_client::Application;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

/// Gets the window object with websys
pub fn get_window() -> web_sys::Window {
	web_sys::window().expect("no global `window` exists")
}

/// Gets the document object with websys
pub fn get_document() -> web_sys::Document {
	get_window().document().expect("should have a document on window")
}

/// Requests an animation frame from websys. This has to be called every frame if you want another frame.
pub fn request_animation_frame(f: &Closure<dyn FnMut(f64)>) {
	get_window().request_animation_frame(f.as_ref().unchecked_ref()).expect("Failed to register `requestAnimationFrame`");
}

/// Adds an event listner to a specific target with a closure with no arguments
pub fn add_event_listener(target: &web_sys::EventTarget, event: &str, f: Closure<dyn FnMut()>) {
	target
		.add_event_listener_with_callback(event, f.as_ref().unchecked_ref())
		.expect(&format!("Failed to register `{event}`"));
	// Leak memory :)
	f.forget();
}

/// Adds an event listner on the document that sends an event
pub fn add_document_event<T, F>(event_name: &'static str, callback: F)
where
	T: wasm_bindgen::convert::FromWasmAbi + 'static,
	F: Fn(&mut Application, T) -> geonext_client::EventType + 'static,
{
	let closure = Closure::wrap(Box::new(move |e: T| {
		crate::APPLICATION_CELL.with(|x| {
			if let Ok(mut application) = x.try_borrow_mut() {
				if let Some(application) = &mut *application {
					let event = callback(application, e);
					application.event(event);
				}
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
pub fn loading_status(status: &str) {
	if let Some(el) = get_document().get_element_by_id("loadingcomponent") {
		el.set_inner_html(status);
	}
}

/// When a panic occurs, notify the user and log the error to the JS console
pub fn panic_hook(info: &core::panic::PanicInfo) {
	// Skip if we have already panicked
	if crate::HAS_CRASHED.with(|cell| cell.replace(true)) {
		return;
	}
	error!("{}", info);

	let document = get_document();
	if let Some(el) = document.get_element_by_id("errorreason") {
		el.set_inner_html(&format!("{info}"));
	}
}
