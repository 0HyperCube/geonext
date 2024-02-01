use crate::html;
use geonext_client::{Application, EventType};
use wasm_bindgen::prelude::*;

pub fn add_events(window: web_sys::Window, canvas: web_sys::HtmlCanvasElement) {
	let resize_closure = Closure::wrap(Box::new(move || {
		let window = html::get_window();
		let width = window.inner_width().unwrap().as_f64().unwrap() as u32;
		let height = window.inner_height().unwrap().as_f64().unwrap() as u32;
		canvas.set_width(width);
		canvas.set_height(height);
		crate::APPLICATION_CELL.with(|cell| {
			if let Ok(mut application) = cell.try_borrow_mut() {
				if let Some(application) = &mut *application {
					application.resize(width, height);
				}
			}
		});
	}) as Box<dyn FnMut()>);
	html::add_event_listener(&window, "resize", resize_closure);

	fn update_pointer_position(application: &mut Application, e: &web_sys::PointerEvent) -> geonext_client::IVec2 {
		application.game_state.input.using_touch = e.pointer_type() == "touch";
		if application.game_state.input.using_touch {
			panic!("Touch is not yet supported. Please implement it yourself");
		}
		application.game_state.input.update_mouse(e.x(), e.y(), e.buttons())
	}
	html::add_document_event("pointerdown", |application, e| {
		update_pointer_position(application, &e);
		EventType::PointerDown(geonext_client::MouseButton::from_num(e.button()))
	});
	html::add_document_event("pointerup", |application, e| {
		update_pointer_position(application, &e);
		EventType::PointerUp(geonext_client::MouseButton::from_num(e.button()))
	});
	html::add_document_event("pointermove", |application, e| EventType::PointerMove(update_pointer_position(application, &e)));

	let wheel_callback = |_: &mut Application, e: web_sys::WheelEvent| EventType::PointerScroll((e.delta_x() as f32, e.delta_y() as f32).into());
	html::add_document_event("wheel", wheel_callback);

	let key_down = |_: &mut Application, e: web_sys::KeyboardEvent| EventType::KeyDown(e.key());
	html::add_document_event("keydown", key_down);

	let key_up = |_: &mut Application, e: web_sys::KeyboardEvent| EventType::KeyUp(e.key());
	html::add_document_event("keyup", key_up);
}
