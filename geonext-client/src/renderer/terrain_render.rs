use super::program::Program;
use crate::{ErrorKind, GameState};
use glam::{Mat4, Vec4};
use glow::{Context, HasContext};
use std::rc::Rc;

pub struct TerrainRender {
	vertex_array: <glow::Context as glow::HasContext>::VertexArray,
	vertex_buffer: <glow::Context as glow::HasContext>::Buffer,
	indicies_count: usize,
	context: Rc<glow::Context>,
}

impl TerrainRender {
	pub unsafe fn new(context: Rc<Context>, verts: &[f32], indices: &[u32]) -> Result<Self, ErrorKind> {
		let indicies_count = indices.len();
		let (_, indices_data, _) = indices.align_to();
		let (_, vert_data, _) = verts.align_to();

		// Create buffers

		let vertex_array = context.create_vertex_array().map_err(ErrorKind::VertexArray)?;
		let vertex_buffer = context.create_buffer().map_err(ErrorKind::VertexArray)?;
		let indices_buffer = context.create_buffer().map_err(ErrorKind::IndexArray)?;

		// bind the Vertex Array Object first, then bind and set vertex buffer(s), and then configure vertex attributes(s).

		context.bind_vertex_array(Some(vertex_array));

		context.bind_buffer(glow::ARRAY_BUFFER, Some(vertex_buffer));
		context.buffer_data_u8_slice(glow::ARRAY_BUFFER, vert_data, glow::STATIC_DRAW);

		context.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(indices_buffer));
		context.buffer_data_u8_slice(glow::ELEMENT_ARRAY_BUFFER, indices_data, glow::STATIC_DRAW);

		//context.vertex_attrib_3_f32(0, 0., 0., 0.);
		context.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, core::mem::size_of::<f32>() as i32 * 6, 0);
		context.enable_vertex_attrib_array(0);

		context.vertex_attrib_pointer_f32(1, 3, glow::FLOAT, false, core::mem::size_of::<f32>() as i32 * 6, core::mem::size_of::<f32>() as i32 * 3);
		context.enable_vertex_attrib_array(1);

		// Unbind buffers
		context.bind_buffer(glow::ARRAY_BUFFER, None);
		context.bind_vertex_array(None);

		Ok(Self {
			vertex_array,
			indicies_count,
			vertex_buffer,
			context,
		})
	}

	pub unsafe fn render(&self, scene_program: &Program, game_state: &GameState) {
		self.context.clear_color(0.207843137, 0.207843137, 0.207843137, 1.);
		self.context.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);

		scene_program.bind();
		scene_program.set_vec4("addColour", Vec4::ZERO);

		let projection = game_state.projection_mat();
		let view = game_state.view_mat();
		let model = Mat4::from_rotation_x(0.);

		scene_program.set_mat4("projection", projection);
		scene_program.set_mat4("view", view);
		scene_program.set_mat4("model", model);

		self.context.bind_vertex_array(Some(self.vertex_array));
		self.context.draw_elements(glow::TRIANGLES, self.indicies_count as i32, glow::UNSIGNED_INT, 0);
	}
}

impl Drop for TerrainRender {
	fn drop(&mut self) {
		unsafe { self.context.delete_vertex_array(self.vertex_array) };
		unsafe { self.context.delete_buffer(self.vertex_buffer) };
	}
}
