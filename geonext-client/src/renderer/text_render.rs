use super::program::Program;
use super::text::{FontCache, TextLayoutCache};
use super::ui_layout::*;
use crate::{ErrorKind, GameState};
use glam::Mat4;
use glow::{Context, HasContext};
use std::rc::Rc;

pub struct TextRender {
	vertex_array: <glow::Context as glow::HasContext>::VertexArray,
	vertex_buffer: <glow::Context as glow::HasContext>::Buffer,
	instance_buffer: <glow::Context as glow::HasContext>::Buffer,
	context: Rc<glow::Context>,
}

impl TextRender {
	pub unsafe fn new(context: Rc<Context>) -> Result<Self, ErrorKind> {
		// Bind text vertex array
		let vertex_array = context.create_vertex_array().map_err(ErrorKind::VertexArray)?;
		context.bind_vertex_array(Some(vertex_array));

		// Bind text vertex buffer
		let vertex_buffer = context.create_buffer().map_err(ErrorKind::VertexArray)?;
		context.bind_buffer(glow::ARRAY_BUFFER, Some(vertex_buffer));
		let square_verts = [(0., 1.), (0_f32, 0_f32), (1., 0.), (0., 1.), (1., 0.), (1., 1.)];
		let (_, src_data, _) = square_verts.align_to();
		context.buffer_data_size(glow::ARRAY_BUFFER, std::mem::size_of::<f32>() as i32 * 6 * 2, glow::STATIC_DRAW);
		context.enable_vertex_attrib_array(0);
		context.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, core::mem::size_of::<f32>() as i32 * 2, 0);
		context.buffer_sub_data_u8_slice(glow::ARRAY_BUFFER, 0, src_data);
		context.bind_buffer(glow::ARRAY_BUFFER, None);

		// Text instance buffer
		let instance_buffer = context.create_buffer().map_err(ErrorKind::VertexArray)?;
		context.bind_buffer(glow::ARRAY_BUFFER, Some(instance_buffer));
		//context.buffer_data_size(glow::ARRAY_BUFFER, 4 * 8, glow::DYNAMIC_DRAW);

		context.vertex_attrib_pointer_f32(1, 4, glow::FLOAT, false, core::mem::size_of::<f32>() as i32 * 8, 0);
		context.enable_vertex_attrib_array(1);
		context.vertex_attrib_divisor(1, 1);

		context.vertex_attrib_pointer_f32(2, 4, glow::FLOAT, false, core::mem::size_of::<f32>() as i32 * 8, core::mem::size_of::<f32>() as i32 * 4);
		context.enable_vertex_attrib_array(2);
		context.vertex_attrib_divisor(2, 1);
		context.bind_buffer(glow::ARRAY_BUFFER, None);

		// Unbind vertex array
		context.bind_vertex_array(None);

		// Create texture
		// s t and r axis = x y z
		// context.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::MIRRORED_REPEAT as i32);
		// context.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::MIRRORED_REPEAT as i32);
		// context.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);

		// Store state
		Ok(Self {
			vertex_array,
			vertex_buffer,
			instance_buffer,
			context,
		})
	}

	pub unsafe fn render(&self, text_program: &Program, game_state: &GameState, font: &mut FontCache) {
		let projection = Mat4::orthographic_rh_gl(0., game_state.viewport.x as f32, game_state.viewport.y as f32, 0., 0., 1000.);
		text_program.bind();
		text_program.set_vec3("textColour", glam::Vec3::ONE);
		text_program.set_mat4("projection", projection);
		self.context.active_texture(glow::TEXTURE0);
		self.context.bind_vertex_array(Some(self.vertex_array));

		// let pos = glam::Vec2::new(game_state.viewport.x as f32 - 200., 20.);
		// let framerate = Text::new(font, &format!("Peek: {}ms", game_state.time.peak_frametime().round()), "regular");
		// framerate.render_glyphs(font, pos, 0.3, self.instance_buffer);

		let debug_info = Container {
			child: Flex {
				children: (
					TextNode::new(font, &"GeoNext Alpha", "regular", 1.),
					TextNode::new(font, &format!("Peek: {}ms", game_state.time.peak_frametime().round()), "regular", 1.),
				),
				main_axis_alignment: MainAxisAlignment::SpaceBetween,
				..default()
			},
			margin: 10.,
			..default()
		};
		let tooltip = Tooltip {
			child: Container {
				child: TextNode::new(font, game_state.map.hovered_name(), "regular", 1.),
				margin: 10.,
				..default()
			},
			mouse: game_state.input.mouse_pos.as_dvec2(),
			..default()
		};
		let mut frame_time = Stack {
			children: (debug_info, tooltip),
			..default()
		};
		frame_time.layout(BoxConstraint::loose(game_state.viewport.as_dvec2()));
		frame_time.render(
			glam::DVec2::ZERO,
			&mut UiRenderer {
				cache: font,
				instances: self.instance_buffer,
			},
		);

		self.context.bind_vertex_array(None);
		self.context.bind_texture(glow::TEXTURE_2D, None);
	}
}

impl Drop for TextRender {
	fn drop(&mut self) {
		unsafe { self.context.delete_vertex_array(self.vertex_array) };
		unsafe { self.context.delete_buffer(self.vertex_buffer) };
		unsafe { self.context.delete_buffer(self.instance_buffer) };
	}
}
