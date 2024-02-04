use super::program::Program;
use crate::{ErrorKind, GameState};
use glam::{Mat4, Vec3, Vec4};
use glow::{Context, HasContext};
use std::rc::Rc;

pub struct SceneRender {
	vertex_array: <glow::Context as glow::HasContext>::VertexArray,
	vertex_buffer: <glow::Context as glow::HasContext>::Buffer,
	indicies_count: usize,
	instances_buffer: <glow::Context as glow::HasContext>::Buffer,
	context: Rc<glow::Context>,
}

impl SceneRender {
	pub unsafe fn new(context: Rc<Context>, verts: &[u8], indices: &[u8]) -> Result<Self, ErrorKind> {
		// Create buffers

		let vertex_array = context.create_vertex_array().map_err(ErrorKind::VertexArray)?;
		let vertex_buffer = context.create_buffer().map_err(ErrorKind::VertexArray)?;
		let indices_buffer = context.create_buffer().map_err(ErrorKind::IndexArray)?;
		let instances_buffer = context.create_buffer().map_err(ErrorKind::InstanceArray)?;

		// bind the Vertex Array Object first, then bind and set vertex buffer(s), and then configure vertex attributes(s).

		context.bind_vertex_array(Some(vertex_array));

		context.bind_buffer(glow::ARRAY_BUFFER, Some(vertex_buffer));
		context.buffer_data_u8_slice(glow::ARRAY_BUFFER, verts, glow::STATIC_DRAW);

		context.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(indices_buffer));
		context.buffer_data_u8_slice(glow::ELEMENT_ARRAY_BUFFER, indices, glow::STATIC_DRAW);

		let stride = core::mem::size_of::<f32>() as i32 * 9;
		for i in 0..3 {
			context.vertex_attrib_pointer_f32(i, 3, glow::FLOAT, false, stride, core::mem::size_of::<f32>() as i32 * i as i32 * 3);
			context.enable_vertex_attrib_array(i);
		}

		// Instances
		context.bind_buffer(glow::ARRAY_BUFFER, Some(instances_buffer));
		let t = [Vec3::ZERO, Vec3::X * 100.];
		let (_, translations, _) = t.align_to();
		context.buffer_data_u8_slice(glow::ARRAY_BUFFER, translations, glow::DYNAMIC_DRAW);
		context.vertex_attrib_pointer_f32(3, 3, glow::FLOAT, false, core::mem::size_of::<f32>() as i32 * 3, 0);
		context.enable_vertex_attrib_array(3);
		context.vertex_attrib_divisor(3, 1);

		// Unbind buffers
		context.bind_buffer(glow::ARRAY_BUFFER, None);
		context.bind_vertex_array(None);
		Ok(Self {
			vertex_array,
			indicies_count: indices.len() / core::mem::size_of::<u32>() as usize,
			vertex_buffer,
			instances_buffer,
			context,
		})
	}

	pub unsafe fn render(&self, scene_program: &Program, game_state: &GameState, translations: &[Vec3]) {
		scene_program.bind();
		scene_program.set_vec4("addColour", Vec4::ZERO);

		let projection = game_state.projection_mat();
		let view = game_state.view_mat();
		let model = Mat4::from_rotation_x(0.);

		scene_program.set_mat4("projection", projection);
		scene_program.set_mat4("view", view);
		scene_program.set_mat4("model", model);

		self.context.bind_vertex_array(Some(self.vertex_array));

		self.context.bind_buffer(glow::ARRAY_BUFFER, Some(self.instances_buffer));
		let (_, translations_cast, _) = translations.align_to();
		self.context.buffer_data_u8_slice(glow::ARRAY_BUFFER, translations_cast, glow::DYNAMIC_DRAW);

		self.context
			.draw_elements_instanced(glow::TRIANGLES, self.indicies_count as i32, glow::UNSIGNED_INT, 0, translations.len() as i32);
	}
}

impl Drop for SceneRender {
	fn drop(&mut self) {
		unsafe { self.context.delete_vertex_array(self.vertex_array) };
		unsafe { self.context.delete_buffer(self.vertex_buffer) };
	}
}
