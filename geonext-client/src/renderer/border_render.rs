use super::program::Program;
use crate::{map::Map, ErrorKind, GameState};
use geonext_shared::{
	map_loader::{Channel, HeightMap, HexCoord},
	territories::CountryId,
};
use glam::{Mat4, UVec2, Vec2, Vec3};
use glow::{Context, HasContext};
use std::{collections::HashSet, rc::Rc};

pub struct BorderRender {
	vertex_array: <glow::Context as glow::HasContext>::VertexArray,
	vertex_buffer: <glow::Context as glow::HasContext>::Buffer,
	context: Rc<glow::Context>,
	verts: Vec<(u32, Vec3, Vec3, Vec3, Vec2)>,
	indices: Vec<u32>,
}

impl BorderRender {
	fn trace_country(&mut self, start_hex: HexCoord, map: &Map, index: &mut u32, country_id: CountryId, visited: &mut HashSet<HexCoord>) {
		let [primary, secondary] = country_id.colours();
		let mut direction = HexCoord::TOP_LEFT;
		let mut current_hex = start_hex;
		let mut current_height = HeightMap::elevation_to_z(map.height_map.sample_at(Channel::TOPO, current_hex.to_offset().as_uvec2()));

		let mut started = false;
		let start_index = *index;
		let indices_start = self.indices.len();
		while !(direction == HexCoord::TOP_LEFT && current_hex == start_hex) || !started {
			let start_corner = current_hex.corner(direction.rotate_anticlockwise());
			let middle_corner = current_hex.corner(direction);
			let middle_height = current_height;

			let proposed_offset = (current_hex + direction).to_offset();
			if map.height_map.in_bounds(proposed_offset) && country_id == map.borders.country_id(proposed_offset.as_uvec2()) {
				current_hex = current_hex + direction;
				current_height = HeightMap::elevation_to_z(map.height_map.sample_at(Channel::TOPO, current_hex.to_offset().as_uvec2()));
				direction = direction.rotate_anticlockwise();
			} else {
				direction = direction.rotate_clockwise();
			}

			let end_corner = current_hex.corner(direction);
			let real_direction = ((end_corner - middle_corner).normalize() + (middle_corner - start_corner).normalize()) / 2.;
			let perpendicular_direction = (Vec2::new(-real_direction.y, real_direction.x) * 0.2).extend(0.);

			let country = country_id.0 as u32;
			self.verts.extend_from_slice(&[
				(country, middle_corner.extend(middle_height), primary, secondary, Vec2::ZERO),
				(country, (middle_corner.extend(middle_height) + perpendicular_direction), primary, secondary, Vec2::ONE),
				(country, middle_corner.extend(current_height), primary, secondary, Vec2::ZERO),
				(country, (middle_corner.extend(current_height) + perpendicular_direction), primary, secondary, Vec2::ONE),
			]);

			if started {
				self.indices.extend([*index - 2, *index - 1, *index, *index, *index - 1, *index + 1]);
				self.indices.extend([*index, *index + 1, *index + 2, *index + 2, *index + 1, *index + 3]);
			}
			*index += 4;

			if direction == HexCoord::TOP_LEFT {
				visited.insert(current_hex);
			}
			started = true;
		}
		if *index != start_index {
			self.indices.extend([start_index, start_index + 1, *index - 2, *index - 2, start_index + 1, *index - 1]);
		}
	}

	fn map_gen(&mut self, map: &Map) {
		self.verts.clear();
		self.indices.clear();
		let mut index = 0;

		let mut visited = HashSet::new();
		for y in 0..map.borders.height() {
			let mut previous_country_id = None;
			for x in 0..map.borders.width() {
				let country_id = map.borders.country_id(UVec2::new(x, y));
				if Some(country_id) == previous_country_id {
					continue;
				}
				previous_country_id = Some(country_id);
				if country_id == CountryId::SEA {
					continue;
				}

				let start_hex = HexCoord::from_offset(x as i32, y as i32);
				if visited.contains(&start_hex) {
					continue;
				}

				self.trace_country(start_hex, map, &mut index, country_id, &mut visited);
			}
		}
	}

	pub unsafe fn new(context: Rc<Context>, map: &Map) -> Result<Self, ErrorKind> {
		info!("Width {} height {}", map.height_map.width, map.height_map.height);

		// Create buffers

		let vertex_array = context.create_vertex_array().map_err(ErrorKind::VertexArray)?;
		let vertex_buffer = context.create_buffer().map_err(ErrorKind::VertexArray)?;
		let indices_buffer = context.create_buffer().map_err(ErrorKind::IndexArray)?;

		let mut border = Self {
			vertex_array,
			vertex_buffer,
			context: context.clone(),
			verts: Vec::new(),
			indices: Vec::new(),
		};
		border.map_gen(map);

		let (_, indices_data, _) = border.indices.align_to();
		let (s, vert_data, e) = border.verts.align_to();
		assert_eq!(s.len(), 0);
		assert_eq!(e.len(), 0);
		let stride = core::mem::size_of::<f32>() as i32 * 12;
		assert_eq!(vert_data.len(), border.verts.len() * stride as usize);

		// bind the Vertex Array Object first, then bind and set vertex buffer(s), and then configure vertex attributes(s).

		context.bind_vertex_array(Some(vertex_array));

		context.bind_buffer(glow::ARRAY_BUFFER, Some(vertex_buffer));
		context.buffer_data_u8_slice(glow::ARRAY_BUFFER, vert_data, glow::STATIC_DRAW);

		context.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(indices_buffer));
		context.buffer_data_u8_slice(glow::ELEMENT_ARRAY_BUFFER, indices_data, glow::STATIC_DRAW);

		context.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, stride, core::mem::size_of::<f32>() as i32 * 1);
		context.enable_vertex_attrib_array(0);

		context.vertex_attrib_pointer_f32(1, 3, glow::FLOAT, true, stride, core::mem::size_of::<f32>() as i32 * 4);
		context.enable_vertex_attrib_array(1);

		context.vertex_attrib_pointer_f32(2, 3, glow::FLOAT, true, stride, core::mem::size_of::<f32>() as i32 * 7);
		context.enable_vertex_attrib_array(2);

		context.vertex_attrib_pointer_f32(3, 2, glow::FLOAT, true, stride, core::mem::size_of::<f32>() as i32 * 10);
		context.enable_vertex_attrib_array(3);

		context.vertex_attrib_pointer_i32(4, 1, glow::UNSIGNED_INT, stride, 0);
		context.enable_vertex_attrib_array(4);

		// Unbind buffers
		context.bind_buffer(glow::ARRAY_BUFFER, None);
		context.bind_vertex_array(None);

		Ok(border)
	}

	pub unsafe fn render(&self, scene_program: &Program, game_state: &GameState) {
		scene_program.bind();
		let projection = game_state.projection_mat();
		let view = game_state.view_mat();
		let model = Mat4::from_rotation_x(0.);

		scene_program.set_mat4("projection", projection);
		scene_program.set_mat4("view", view);
		scene_program.set_mat4("model", model);
		scene_program.set_uint("select", game_state.map.hovered_country().0 as u32);

		self.context.bind_vertex_array(Some(self.vertex_array));
		self.context.draw_elements(glow::TRIANGLES, self.indices.len() as i32, glow::UNSIGNED_INT, 0);
	}
}

impl Drop for BorderRender {
	fn drop(&mut self) {
		unsafe { self.context.delete_vertex_array(self.vertex_array) };
		unsafe { self.context.delete_buffer(self.vertex_buffer) };
	}
}
