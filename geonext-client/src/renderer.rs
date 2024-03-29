use std::rc::Rc;

use geonext_shared::map_loader::HexCoord;
use glam::Vec3;
use glow::{Context, HasContext};

use crate::{ErrorKind, GameState};

mod program;
use program::*;

use self::{border_render::BorderRender, terrain_render::SceneRender, text_render::TextRender};
mod atlas;
mod border_render;
mod terrain_render;
pub mod text;
mod text_render;
#[allow(dead_code)]
pub mod ui_layout;

pub struct Programs {
	scene_program: Program,
	border_program: Program,
	_ui_program: Program,
	text_program: Program,
}

impl Programs {
	fn load_shaders(context: &Rc<Context>) -> Result<Self, ErrorKind> {
		let frag = Shader::new(context.clone(), glow::FRAGMENT_SHADER, include_str!("../assets/shaders/scene-fs.glsl"))?;
		let vert = Shader::new(context.clone(), glow::VERTEX_SHADER, include_str!("../assets/shaders/scene-vs.glsl"))?;
		let scene_program = Program::new(context.clone(), &[frag, vert], &[])?;

		let frag = Shader::new(context.clone(), glow::FRAGMENT_SHADER, include_str!("../assets/shaders/border-fs.glsl"))?;
		let vert = Shader::new(context.clone(), glow::VERTEX_SHADER, include_str!("../assets/shaders/border-vs.glsl"))?;
		let border_program = Program::new(context.clone(), &[frag, vert], &[])?;

		let frag = Shader::new(context.clone(), glow::FRAGMENT_SHADER, include_str!("../assets/shaders/text-fs.glsl"))?;
		let vert = Shader::new(context.clone(), glow::VERTEX_SHADER, include_str!("../assets/shaders/text-vs.glsl"))?;
		let text_program = Program::new(context.clone(), &[frag, vert], &[])?;

		let frag = Shader::new(context.clone(), glow::FRAGMENT_SHADER, include_str!("../assets/shaders/text-fs.glsl"))?;
		let vert = Shader::new(context.clone(), glow::VERTEX_SHADER, include_str!("../assets/shaders/text-vs.glsl"))?;
		let ui_program = Program::new(context.clone(), &[frag, vert], &[])?;

		Ok(Programs {
			scene_program,
			border_program,
			_ui_program: ui_program,
			text_program,
		})
	}
}

/// Contains the glow opengl state
pub struct OpenGl {
	terrain: Option<SceneRender>,
	sawmill: Option<SceneRender>,
	mine: Option<SceneRender>,
	farm: Option<SceneRender>,
	army: Option<SceneRender>,
	text: Option<TextRender>,
	border: Option<BorderRender>,
	programs: Option<Programs>,
	//framebuffers: FnvHashMap<ImageId, Result<Framebuffer, ErrorKind>>,
	context: Rc<glow::Context>,
	pub font: text::FontCache,
}

impl OpenGl {
	/// Construct a new opengl state based on the specified context
	pub fn new(context: glow::Context) -> Self {
		let context = Rc::new(context);
		Self {
			terrain: None,
			sawmill: None,
			farm: None,
			mine: None,
			army: None,
			text: None,
			border: None,
			programs: None,
			context: context.clone(),
			font: text::FontCache::new(context),
		}
	}

	fn setup_opengl(&mut self) {
		unsafe {
			self.context.enable(glow::DEPTH_TEST);
			self.context.enable(glow::BLEND);
			self.context.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
			info!(
				"Shading language version: {}\nMax vertex attributes: {}\nVendor: {}\nVersion: {}",
				self.context.get_parameter_string(glow::SHADING_LANGUAGE_VERSION),
				self.context.get_parameter_i32(glow::MAX_VERTEX_ATTRIBS),
				self.context.get_parameter_string(glow::VENDOR),
				self.context.get_parameter_string(glow::VERSION),
			);
		};
	}

	/// Initalise opengl
	pub fn init(&mut self, verts: &[f32], indices: &[u32], game_state: &GameState) -> Result<(), ErrorKind> {
		self.setup_opengl();
		self.font.init()?;
		self.programs = Some(Programs::load_shaders(&self.context)?);

		unsafe {
			let (_, indices_data, _) = indices.align_to();
			let (_, vert_data, _) = verts.align_to();
			self.terrain = Some(SceneRender::new(self.context.clone(), vert_data, indices_data)?);
		}

		let to_scene = |dat: &[u8]| {
			let len_vert = u32::from_le_bytes([dat[0], dat[1], dat[2], dat[3]]) as usize;
			unsafe { SceneRender::new(self.context.clone(), &dat[8..][..len_vert], &dat[8..][len_vert..]) }
		};

		self.sawmill = Some(to_scene(include_bytes!("./../../assets/dat/sawmill.dat"))?);
		self.farm = Some(to_scene(include_bytes!("./../../assets/dat/farm.dat"))?);
		self.mine = Some(to_scene(include_bytes!("./../../assets/dat/mine.dat"))?);
		self.army = Some(to_scene(include_bytes!("./../../assets/dat/army.dat"))?);

		unsafe { self.text = Some(TextRender::new(self.context.clone())?) };
		unsafe { self.border = Some(BorderRender::new(self.context.clone(), &game_state.map)?) };

		Ok(())
	}

	/// Renders a frame
	pub fn rerender(&mut self, game_state: &GameState) {
		let Some(Programs {
			scene_program,
			border_program,
			_ui_program: _,
			text_program,
		}) = &self.programs
		else {
			return;
		};
		if game_state.map.updated {
			match unsafe { BorderRender::new(self.context.clone(), &game_state.map) } {
				Ok(borders) => self.border = Some(borders),
				Err(e) => error!("Error generating borders {e}"),
			}
		}
		unsafe {
			self.context.clear_color(28. / 255., 27. / 255., 34. / 255., 1.);
			self.context.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
		}
		if let Some(terrain) = &self.terrain {
			unsafe { terrain.render(&scene_program, game_state, &[Vec3::ZERO]) };
		}

		if let Some(border) = &self.border {
			unsafe { border.render(&border_program, game_state) };
		}
		if let Some(sawmill) = &self.sawmill {
			let val = (0..game_state.map.height_map.height)
				.flat_map(|y| (0..game_state.map.height_map.width).map(move |x| game_state.map.height_map.hex_centre(x, y)))
				.collect::<Vec<_>>();

			unsafe { sawmill.render(&scene_program, game_state, &val) };
		}

		// UI must be last so it doesn't cause artifact
		if let Some(text) = &self.text {
			unsafe { text.render(&text_program, game_state, &mut self.font) };
		}
	}

	/// Resizes the viewport
	pub fn resize(&mut self, game_state: &GameState) {
		unsafe {
			self.context.viewport(0, 0, game_state.viewport.x as i32, game_state.viewport.y as i32);
		}
	}
}
