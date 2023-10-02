use super::program::Program;
use crate::{
	map::{Channel, HexCoord, Map},
	ErrorKind, GameState,
};
use glam::{IVec2, Mat4, UVec2, Vec2, Vec3, Vec4};
use glow::{Context, HasContext};
use std::{collections::HashSet, rc::Rc};

pub struct BorderRender {
	vertex_array: <glow::Context as glow::HasContext>::VertexArray,
	vertex_buffer: <glow::Context as glow::HasContext>::Buffer,
	indicies_count: usize,
	context: Rc<glow::Context>,
}

impl BorderRender {
	fn map_gen(map: &Map) -> (Vec<(Vec3, Vec3, Vec3, Vec2)>, Vec<u32>) {
		let mut indices = Vec::new();

		let primary = Vec3::new(1., 0., 0.);
		let secondary = Vec3::new(0., 0., 1.);
		let mut verts = Vec::new();
		let mut index = 0;
		let z_fight = Vec3::new(0., 0., 0.01);

		let mut visited = HashSet::new();
		for y in 0..map.height {
			let mut previous_name_index = None;
			for x in 0..map.width {
				let name_index = map.sample_at(Channel::NAME, UVec2::new(x, y));
				if Some(name_index) == previous_name_index {
					continue;
				}
				previous_name_index = Some(name_index);
				let start_hex = HexCoord::from_offset(x as i32, y as i32);
				if visited.contains(&start_hex) {
					continue;
				}

				let mut direction = HexCoord::TOP_LEFT;
				let mut current_hex = start_hex;
				let mut current_height = Map::elevation_to_z(map.sample_at(Channel::TOPO, current_hex.to_offset().as_uvec2()));

				info!("New hex");
				let mut started = false;
				while !(direction == HexCoord::TOP_LEFT && current_hex == start_hex) || !started {
					let start_corner = current_hex.corner(direction, current_height);
					//info!("dir {direction:?} current {current_hex:?}");
					let proposed_offset = (current_hex + direction).to_offset();
					if map.in_bounds(proposed_offset) && name_index == map.sample_at(Channel::NAME, proposed_offset.as_uvec2()) {
						current_hex = current_hex + direction;
						current_height = Map::elevation_to_z(map.sample_at(Channel::TOPO, current_hex.to_offset().as_uvec2()));
						direction = direction.rotate_anticlockwise();
					} else {
						direction = direction.rotate_clockwise();
					}
					let end_corner = current_hex.corner(direction, current_height);
					let real_direction = (end_corner.truncate() - start_corner.truncate()).normalize();
					let perp_direction = Vec2::new(real_direction.y, -real_direction.x) * 0.2;
					verts.push((start_corner + z_fight, primary, secondary, Vec2::new(0., 0.)));
					verts.push((start_corner + perp_direction.extend(0.) + z_fight, primary, secondary, Vec2::new(1., 1.)));
					if started {
						indices.extend([index - 2, index - 1, index, index, index - 1, index + 1]);
					}
					index += 4;
					verts.push((start_corner.truncate().extend(current_height) + z_fight, primary, secondary, Vec2::new(0., 0.)));
					verts.push((
						start_corner.truncate().extend(current_height) + perp_direction.extend(0.) + z_fight,
						primary,
						secondary,
						Vec2::new(1., 1.),
					));

					if direction == HexCoord::TOP_LEFT {
						visited.insert(current_hex);
					}
					started = true;
				}
			}
		}
		(verts, indices)
	}

	pub unsafe fn new(context: Rc<Context>, map: &Map) -> Result<Self, ErrorKind> {
		info!("Width {} height {}", map.width, map.height);
		let (verts, indices) = Self::map_gen(map);

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
		context.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, core::mem::size_of::<f32>() as i32 * 11, 0);
		context.enable_vertex_attrib_array(0);

		context.vertex_attrib_pointer_f32(1, 3, glow::FLOAT, true, core::mem::size_of::<f32>() as i32 * 11, core::mem::size_of::<f32>() as i32 * 3);
		context.enable_vertex_attrib_array(1);
		context.vertex_attrib_pointer_f32(2, 3, glow::FLOAT, true, core::mem::size_of::<f32>() as i32 * 11, core::mem::size_of::<f32>() as i32 * 6);
		context.enable_vertex_attrib_array(2);
		context.vertex_attrib_pointer_f32(3, 2, glow::FLOAT, true, core::mem::size_of::<f32>() as i32 * 11, core::mem::size_of::<f32>() as i32 * 9);
		context.enable_vertex_attrib_array(3);

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
		scene_program.bind();
		//scene_program.set_vec4("addColour", Vec4::ZERO);

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

impl Drop for BorderRender {
	fn drop(&mut self) {
		unsafe { self.context.delete_vertex_array(self.vertex_array) };
		unsafe { self.context.delete_buffer(self.vertex_buffer) };
	}
}
