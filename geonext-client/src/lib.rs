use std::collections::HashMap;

use geonext_shared::{territories::Territories, ServerMessage};
use glam::Mat4;
pub use glam::{IVec2, UVec2, Vec2};
use renderer::OpenGl;
mod camera;
mod events;
mod map;
mod renderer;
mod terrain;
mod time;
pub use camera::Camera;
pub use events::*;
pub use time::Time;

#[macro_use]
extern crate log;

#[derive(Debug)]
pub struct Assets(pub HashMap<String, Vec<u8>>);

impl Assets {
	pub fn assets() -> &'static [(&'static str, &'static str)] {
		&[("regular", "assets/RobotoSlab-Regular.ttf"), ("heightmap", "assets/heightmap.jpeg"), ("map", "assets/map.txt")]
	}
	pub fn get<'a>(&'a self, asset: &'static str) -> &'a Vec<u8> {
		self.0.get(asset).expect(&format!("Failed to load asset {asset}!"))
	}
	pub fn take(&mut self, asset: &'static str) -> Vec<u8> {
		self.0.remove(asset).expect(&format!("Failed to load asset {asset}!"))
	}
}
#[derive(Debug, Default)]
pub struct GameState {
	pub viewport: UVec2,
	pub scale_factor: f32,
	pub time: Time,
	pub camera: Camera,
	pub terrain: terrain::Terrain,
	pub input: InputSystem,
	pub map: map::Map,
}
impl GameState {
	#[inline]
	pub fn aspect_ratio(&self) -> f32 {
		self.viewport.x as f32 / self.viewport.y as f32
	}
	pub fn init(&mut self, event_layers: &mut EventLayers) {
		self.camera.position = self.terrain.size.as_vec2() / 2.;

		event_layers.push(Self::update_camera);
		event_layers.push(Self::update_map);
		event_layers.push(Self::hover);
	}
	pub fn projection_mat(&self) -> Mat4 {
		Mat4::perspective_rh_gl(45f32.to_radians(), self.aspect_ratio(), 0.1, 1000.)
	}
	pub fn view_mat(&self) -> Mat4 {
		self.camera.to_matrix(&self.terrain)
	}
	fn update_camera(&mut self, event: &EventType) -> bool {
		match event {
			EventType::PointerMove(delta) if self.input.mouse_down(MouseButton::Primary) => {
				self.camera.velocity -= delta.as_vec2();
				true
			}
			EventType::Update => {
				self.camera.move_by(self.camera.velocity);

				if self.input.mouse_down(MouseButton::Primary) {
					self.camera.velocity = Vec2::ZERO;
				} else {
					self.camera.velocity *= 0.95;
				}

				true
			}
			EventType::PointerScroll(delta) => {
				const ZOOM_WHEEL_RATE: f32 = 1. / 600.;

				let scroll = delta.y.signum() as f32 * (delta.y * delta.y + f32::min(delta.y.abs(), delta.x.abs()).powi(2)).sqrt();

				let mut zoom_factor = 1. + scroll.abs() * ZOOM_WHEEL_RATE;
				if delta.y < 0. {
					zoom_factor = 1. / zoom_factor
				};

				let viewport_bounds = self.viewport.as_vec2();
				let new_viewport_bounds = viewport_bounds / zoom_factor;
				let delta_size = viewport_bounds - new_viewport_bounds;
				let mouse_fraction = self.input.mouse_pos.as_vec2() / viewport_bounds;
				let delta = delta_size * (Vec2::splat(0.5) - mouse_fraction);

				self.camera.zoom *= zoom_factor;
				self.camera.move_by(delta);

				true
			}

			_ => false,
		}
	}

	fn update_map(&mut self, event: &EventType) -> bool {
		let EventType::Message(ServerMessage::Map(map)) = event else {
			return false;
		};
		self.map.borders = Territories::from_rle(map);
		self.map.updated = true;
		true
	}

	fn hover(&mut self, event: &EventType) -> bool {
		if let EventType::PointerMove(_delta) = event {
			self.map.update_hover(self.projection_mat(), self.view_mat(), self.input.mouse_pos.as_vec2() / self.viewport.as_vec2());
			true
		} else {
			false
		}
	}
}

pub struct Application {
	pub game_state: GameState,
	renderer: OpenGl,
	assets: Assets,
	input_layers: EventLayers,
}
impl core::fmt::Debug for Application {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Application").field("environment", &self.game_state).field("assets", &self.assets).finish()
	}
}

impl Application {
	/// Constructs a new application based on the specified game state, glow context and assets
	pub fn new(mut game_state: GameState, context: glow::Context, mut assets: Assets) -> Result<Self, ErrorKind> {
		let mut renderer = OpenGl::new(context);
		game_state.map.load(assets.take("map"));
		game_state.terrain.load(&assets.get("heightmap"));
		let (vertices, indices) = game_state.map.height_map.generate_terrain();
		renderer.init(&vertices, &indices, &game_state)?;
		renderer.font.add_font(&assets, "regular");

		let mut input_layers = Default::default();
		game_state.init(&mut input_layers);
		Ok(Self {
			game_state,
			renderer,
			assets,
			input_layers,
		})
	}

	/// Redraws the game
	pub fn update(&mut self, time: f32) {
		self.game_state.time.update_time(time);
		self.event(EventType::Update);
		self.renderer.rerender(&self.game_state);
		self.game_state.map.updated = false;
		//Mat4::orthographic_rh_gl(left, right, bottom, top, near, far)

		// self.canvas.set_size(width, height, dpi_factor as f32);
		// self.canvas.clear_rect(0, 0, width, height, Color::rgbf(0.9, 0.9, 0.9));

		// self.canvas.flush();
	}

	/// Resizes the screen
	pub fn resize(&mut self, x: u32, y: u32) {
		self.game_state.viewport = UVec2::new(x, y);
		self.renderer.resize(&self.game_state);
	}

	/// Handle an event
	pub fn event(&mut self, event: EventType) {
		//info!("Event {event:?} input {:?}", self.game_state.input);
		for layer in &self.input_layers {
			if layer(&mut self.game_state, &event) {
				break;
			}
		}
	}
}

#[derive(Debug)]
pub enum ErrorKind {
	CreateBuffer(String),
	CreateTexture(String),
	ShaderCompileError(String),
	ProgramLinkError(String),
	VertexArray(String),
	IndexArray(String),
	InstanceArray(String),
}

impl core::fmt::Display for ErrorKind {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(
			f,
			"{}",
			match self {
				ErrorKind::CreateBuffer(x) => x,
				ErrorKind::CreateTexture(x) => x,
				ErrorKind::ShaderCompileError(x) => x,
				ErrorKind::ProgramLinkError(x) => x,
				ErrorKind::VertexArray(x) => x,
				ErrorKind::IndexArray(x) => x,
				ErrorKind::InstanceArray(x) => x,
			}
		)
	}
}
