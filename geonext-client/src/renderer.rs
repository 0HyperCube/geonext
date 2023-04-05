use std::rc::Rc;

use glam::{Mat4, Vec4};
use glow::HasContext;

use crate::{ErrorKind, GameState};

mod program;
use program::*;
mod atlas;
pub mod text;

pub struct Programs {
	scene_program: Program,
	_ui_program: Program,
	text_program: Program,
}

/// Contains the glow opengl state
pub struct OpenGl {
	vert_arr: Option<<glow::Context as glow::HasContext>::VertexArray>,
	vert_buff: Option<<glow::Context as glow::HasContext>::Buffer>,
	text_vertex_array: Option<<glow::Context as glow::HasContext>::VertexArray>,
	text_vertex_buffer: Option<<glow::Context as glow::HasContext>::Buffer>,
	indicies_count: usize,
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
			vert_arr: None,
			vert_buff: None,
			text_vertex_array: None,
			text_vertex_buffer: None,
			indicies_count: 0,
			programs: None,
			context: context.clone(),

			font: text::FontCache::new(context),
		}
	}

	/// Initalise opengl
	pub fn init(&mut self, verts: &[f32], indices: &[usize]) -> Result<(), ErrorKind> {
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

		self.font.init()?;

		// Load shaders and link them into a shader program
		let frag = Shader::new(self.context.clone(), glow::FRAGMENT_SHADER, include_str!("../assets/shaders/scene-fs.glsl"))?;
		let vert = Shader::new(self.context.clone(), glow::VERTEX_SHADER, include_str!("../assets/shaders/scene-vs.glsl"))?;
		let scene_program = Program::new(self.context.clone(), &[frag, vert], &[])?;

		let frag = Shader::new(self.context.clone(), glow::FRAGMENT_SHADER, include_str!("../assets/shaders/text-fs.glsl"))?;
		let vert = Shader::new(self.context.clone(), glow::VERTEX_SHADER, include_str!("../assets/shaders/text-vs.glsl"))?;
		let text_program = Program::new(self.context.clone(), &[frag, vert], &[])?;

		let frag = Shader::new(self.context.clone(), glow::FRAGMENT_SHADER, include_str!("../assets/shaders/text-fs.glsl"))?;
		let vert = Shader::new(self.context.clone(), glow::VERTEX_SHADER, include_str!("../assets/shaders/text-vs.glsl"))?;
		let ui_program = Program::new(self.context.clone(), &[frag, vert], &[])?;

		unsafe {
			// debug_assert_eq!(core::mem::size_of::<Vec3>(), core::mem::size_of::<f32>() * 3);
			// Vec3::dot(Vec3::ONE, Vec3::ONE);
			// #[rustfmt::skip]
			// let verts = [
			// 	// Positions                      // Colours
			// 	Vec3::new(0.4, 0.5, 0.),  Vec3::new(1., 0., 0.),
			// 	Vec3::new(0.5, -0.5, 0.), Vec3::new(0., 1., 0.),
			// 	Vec3::new(-0.5, -0.5, 0.),Vec3::new(0., 0., 1.),
			// 	Vec3::new(-0.5, 0.5, 0.), Vec3::new(0., 0., 0.),
			// ];

			//let indices = [0_usize, 1, 3];
			self.indicies_count = indices.len();
			let (_, indices_data, _) = indices.align_to();
			let (_, vert_data, _) = verts.align_to();

			// Create buffers

			let vertex_array = self.context.create_vertex_array().map_err(ErrorKind::VertexArray)?;
			let vertex_buffer = self.context.create_buffer().map_err(ErrorKind::VertexArray)?;
			let indices_buffer = self.context.create_buffer().map_err(ErrorKind::IndexArray)?;

			// bind the Vertex Array Object first, then bind and set vertex buffer(s), and then configure vertex attributes(s).

			self.context.bind_vertex_array(Some(vertex_array));

			self.context.bind_buffer(glow::ARRAY_BUFFER, Some(vertex_buffer));
			self.context.buffer_data_u8_slice(glow::ARRAY_BUFFER, vert_data, glow::STATIC_DRAW);

			self.context.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(indices_buffer));
			self.context.buffer_data_u8_slice(glow::ELEMENT_ARRAY_BUFFER, indices_data, glow::STATIC_DRAW);

			//self.context.vertex_attrib_3_f32(0, 0., 0., 0.);
			self.context.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, core::mem::size_of::<f32>() as i32 * 6, 0);
			self.context.enable_vertex_attrib_array(0);

			self.context
				.vertex_attrib_pointer_f32(1, 3, glow::FLOAT, false, core::mem::size_of::<f32>() as i32 * 6, core::mem::size_of::<f32>() as i32 * 3);
			self.context.enable_vertex_attrib_array(1);

			// Unbind buffers
			self.context.bind_buffer(glow::ARRAY_BUFFER, None);
			self.context.bind_vertex_array(None);

			let text_vertex_array = self.context.create_vertex_array().map_err(ErrorKind::VertexArray)?;
			let text_vertex_buffer = self.context.create_buffer().map_err(ErrorKind::VertexArray)?;

			self.context.bind_vertex_array(Some(text_vertex_array));
			self.context.bind_buffer(glow::ARRAY_BUFFER, Some(text_vertex_buffer));
			self.context.buffer_data_size(glow::ARRAY_BUFFER, std::mem::size_of::<f32>() as i32 * 6 * 4, glow::DYNAMIC_DRAW);
			self.context.enable_vertex_attrib_array(0);
			self.context.vertex_attrib_pointer_f32(0, 4, glow::FLOAT, false, core::mem::size_of::<f32>() as i32 * 4, 0);

			// Unbind buffers
			self.context.bind_buffer(glow::ARRAY_BUFFER, None);
			self.context.bind_vertex_array(None);

			// Create texture
			// s t and r axis = x y z
			// self.context.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::MIRRORED_REPEAT as i32);
			// self.context.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::MIRRORED_REPEAT as i32);
			// self.context.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);

			// Store state
			self.vert_arr = Some(vertex_array);
			self.vert_buff = Some(vertex_buffer);
			self.text_vertex_array = Some(text_vertex_array);
			self.text_vertex_buffer = Some(text_vertex_buffer);
			self.programs = Some(Programs {
				scene_program,
				_ui_program: ui_program,
				text_program,
			});
		}

		Ok(())
	}

	/// Renders a frame
	pub fn rerender(&mut self, game_state: &GameState) {
		if let (Some(vertex_array), Some(text_vertex_array), Some(text_vertex_buffer), Some(programs)) = (self.vert_arr, self.text_vertex_array, self.text_vertex_buffer, &self.programs) {
			let Programs {
				scene_program,
				_ui_program: _,
				text_program,
			} = programs;
			unsafe {
				self.context.clear_color(0.207843137, 0.207843137, 0.207843137, 1.);
				self.context.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);

				scene_program.bind();
				scene_program.set_vec4("addColour", Vec4::ZERO);

				let projection = Mat4::perspective_rh_gl(45f32.to_radians(), game_state.aspect_ratio(), 0.1, 1000.);
				let view = game_state.camera.to_matrix(&game_state.terrain);
				let model = Mat4::from_rotation_x(0.);

				scene_program.set_mat4("projection", projection);
				scene_program.set_mat4("view", view);
				scene_program.set_mat4("model", model);

				self.context.bind_vertex_array(Some(vertex_array));
				self.context.draw_elements(glow::TRIANGLES, self.indicies_count as i32, glow::UNSIGNED_INT, 0);

				let projection = Mat4::orthographic_rh_gl(0., game_state.viewport.x as f32, 0., game_state.viewport.y as f32, 0., 1000.);
				text_program.bind();
				text_program.set_vec3("textColour", glam::Vec3::ONE);
				text_program.set_mat4("projection", projection);
				self.context.active_texture(glow::TEXTURE0);
				self.context.bind_vertex_array(Some(text_vertex_array));
				self.font
					.render_glyphs("the quick brown fox jumped over the lazy dog", "regular", glam::Vec2::splat(50.), text_vertex_buffer);
				self.font.render_glyphs(
					&format!("peek: {}ms", game_state.time.peak_frametime().round()),
					"regular",
					glam::Vec2::new(50., game_state.viewport.y as f32 - 50.),
					text_vertex_buffer,
				);
				self.context.bind_vertex_array(None);
				self.context.bind_texture(glow::TEXTURE_2D, None);
			}
		}
	}

	/// Resizes the viewport
	pub fn resize(&mut self, game_state: &GameState) {
		unsafe {
			self.context.viewport(0, 0, game_state.viewport.x as i32, game_state.viewport.y as i32);
		}
	}
}

impl Drop for OpenGl {
	fn drop(&mut self) {
		if let Some(vertex_array) = self.vert_arr {
			unsafe { self.context.delete_vertex_array(vertex_array) };
		}
		if let Some(buffer) = self.vert_buff {
			unsafe { self.context.delete_buffer(buffer) };
		}
	}
}
