use glam::{IVec2, UVec2, Vec2};

use crate::GameState;

/// An event layer is a function that subscribes to the event stream
pub type EventLayers = Vec<fn(game_state: &mut GameState, event: &EventType) -> bool>;

/// A type of mouse button
#[derive(Debug, Clone, Copy)]
pub enum MouseButton {
	Primary,
	Auxiliary,
	Secondary,
}
impl MouseButton {
	/// Casts a mouse button to a number
	/// following https://developer.mozilla.org/en-US/docs/Web/API/MouseEvent/buttons#value
	pub fn to_bit(&self) -> u16 {
		match self {
			MouseButton::Primary => 1,
			MouseButton::Secondary => 2,
			MouseButton::Auxiliary => 4,
		}
	}

	/// Creates a mouse button from a number
	/// following https://developer.mozilla.org/en-US/docs/Web/API/MouseEvent/button#value
	pub fn from_num(val: i16) -> Self {
		match val {
			0 => Self::Primary,
			1 => Self::Auxiliary,
			2 => Self::Secondary,
			_ => panic!("Invalid mouse"),
		}
	}
}

/// Events are handled immedialty
#[derive(Debug, Clone)]
pub enum EventType {
	PointerDown(MouseButton),
	PointerUp(MouseButton),
	PointerMove(IVec2),
	PointerScroll(Vec2),
	KeyDown(String),
	KeyUp(String),
	Resize(UVec2),
	Update,
}

#[derive(Debug, Default)]
pub struct InputSystem {
	pub mouse_pos: IVec2,
	pub using_touch: bool,

	mouse_buttons: u16,
}
impl InputSystem {
	/// Returns weather a particular mouse button is down
	pub fn mouse_down(&self, button: MouseButton) -> bool {
		(self.mouse_buttons & button.to_bit()) != 0
	}

	/// Updates the mouse position and down, returning the delta movement
	pub fn update_mouse(&mut self, x: i32, y: i32, buttons: u16) -> IVec2 {
		let new = IVec2::new(x, y);
		let mouse_delta = new - self.mouse_pos;
		self.mouse_pos = new;
		self.mouse_buttons = buttons;
		mouse_delta
	}
}
