use std::ops::Range;

use glam::{Mat4, UVec2, Vec2, Vec3};

#[derive(Debug, Default)]
pub struct Map {
	map: Vec<u8>,
	width: u32,
	height: u32,
	channels: u32,
	name_index: Vec<Range<usize>>,
	data_start: usize,
}

#[derive(Clone, Copy)]
pub struct Channel(u32);
impl Channel {
	pub const SNOW: Self = Self(1);
	pub const VEG: Self = Self(2);
	pub const TOPO: Self = Self(3);
}

impl Map {
	const RADII: f32 = 1.;
	const APOTHEM: f32 = 0.8660254037844386;

	pub fn load(&mut self, map: Vec<u8>) {
		self.map = map;
		struct Index<'a>(usize, &'a [u8]);
		impl Index<'_> {
			fn next(&mut self) -> u8 {
				self.0 += 1;
				self.1[self.0 - 1]
			}
		}
		let mut index = Index(0, &self.map);
		self.width = u16::from_le_bytes([index.next(), index.next()]) as u32;
		self.height = u16::from_le_bytes([index.next(), index.next()]) as u32;
		self.channels = u16::from_le_bytes([index.next(), index.next()]) as u32;
		let num_names = index.next();

		self.name_index = (0..num_names)
			.map(|_| {
				let length = index.next() as usize;
				let range = index.0..index.0 + length;
				index.0 += length;
				range
			})
			.collect();
		self.data_start = index.0;

		info!("data start {} Topo {}", self.data_start, self.sample_at(Channel::TOPO, UVec2::new(50, 20)));
	}
	pub fn get_name(&self, index: usize) -> &str {
		if let Some(name_index) = self.name_index.get(index) {
			// Note that python always encodes strings correctly
			unsafe { std::str::from_utf8_unchecked(&self.map[name_index.clone()]) }
		} else {
			"Water"
		}
	}
	pub fn sample_at(&self, channel: Channel, pos: UVec2) -> u8 {
		unsafe { *self.map.get_unchecked(self.data_start + ((pos.x * self.height + pos.y) * self.channels + channel.0) as usize) }
	}

	pub fn sample_left(&self, channel: Channel, pos: UVec2) -> Option<(u8, UVec2)> {
		(pos.x > 0)
			.then(|| self.sample_at(channel, UVec2::new(pos.x - 1, pos.y)))
			.map(|height| (height, UVec2::new(pos.x - 1, pos.y)))
	}

	pub fn sample_up_left(&self, channel: Channel, pos: UVec2) -> Option<(u8, UVec2)> {
		if pos.y > 0 && pos.x >= pos.y % 2 {
			let pos = UVec2::new(pos.x - (pos.y % 2), pos.y - 1);
			Some((self.sample_at(channel, pos), pos))
		} else {
			None
		}
	}
	pub fn sample_up_right(&self, channel: Channel, pos: UVec2) -> Option<(u8, UVec2)> {
		if pos.y > 0 && pos.x < self.width - 1 + pos.y % 2 {
			let pos = UVec2::new(pos.x + 1 - (pos.y % 2), pos.y - 1);
			Some((self.sample_at(channel, pos), pos))
		} else {
			None
		}
	}

	fn xor_rand(mut x: u32) -> u32 {
		x ^= x << 13;
		x ^= x >> 17;
		x ^= x << 5;
		info!("{x}");
		x
	}

	pub fn generate_terrain(&self) -> (Vec<f32>, Vec<u32>) {
		assert!(!self.map.is_empty(), "Map should be populated");

		let vertex_count = (self.width * self.height) as usize * 6 * 6;
		let tri_count = (self.width * self.height) as usize * 4 * 3;
		let mut verticies = Vec::with_capacity(vertex_count);
		let mut tris = Vec::with_capacity(tri_count);

		let push_vert = |colour: Vec3, verticies: &mut Vec<f32>, pos: Vec3| {
			verticies.extend(pos.to_array());
			verticies.extend(colour.to_array());
		};

		let topo = Channel::TOPO;

		for pos in (0..self.height).flat_map(|y| (0..self.width).map(move |x| UVec2::new(x as u32, y as u32))) {
			let centre = Vec2::new(((pos.x * 2 + 1 - (pos.y % 2)) as f32 - 1.) * Map::APOTHEM, pos.y as f32 * (Map::RADII * 3.) / 2.);
			let _snow = self.sample_at(Channel::SNOW, pos);
			let vegitation = self.sample_at(Channel::VEG, pos);
			let elevation = self.sample_at(topo, pos);
			let to_float = |a, b, c| Vec3::new(a as f32, b as f32, c as f32) / 255.;
			let colour = if elevation > 240 {
				to_float(29, 65, 99)
			} else {
				let lerp = |a, b, t| ((a * (1. - t as f32 / 255.)) + (b * (t as f32 / 255.)));

				let vegitation = lerp(to_float(211, 175, 149), to_float(63, 92, 42), if pos.y < 20 { 0 } else { vegitation });
				let offset = (Self::xor_rand((pos.x * pos.y) as u32) as f32 / u32::MAX as f32) * 0.8 - 0.3;
				let up = ((pos.x as f32 / self.width as f32) - 0.5).abs().sqrt() * 0.4;
				info!("Offset {offset}");
				let t = ((1. - ((pos.y as f32 / 30. - offset - up).min(1.)).powi(4)) * 255.) as u8;
				lerp(vegitation, Vec3::ONE, t.saturating_add(((elevation as f32 / 255.).powi(4) * 255.) as u8))
			};
			//let colour = [0., 0., 0.];
			let height = (if elevation > 240 { -0.01 } else { elevation as f32 / 255. }) * 5.;
			let top_pos = (centre + Vec2::new(0., -Map::RADII)).extend(height);

			let top_left_pos = (centre + Vec2::new(-Map::APOTHEM, -Map::RADII / 2.)).extend(height);
			let top_right_pos = (centre + Vec2::new(Map::APOTHEM, -Map::RADII / 2.)).extend(height);
			let bottom_left_pos = (centre + Vec2::new(-Map::APOTHEM, Map::RADII / 2.)).extend(height);
			let bottom_right_pos = (centre + Vec2::new(Map::APOTHEM, Map::RADII / 2.)).extend(height);
			let bottom_pos = (centre + Vec2::new(0., Map::RADII)).extend(height);

			push_vert(colour, &mut verticies, top_pos);
			push_vert(colour, &mut verticies, top_left_pos);
			push_vert(colour, &mut verticies, top_right_pos);
			push_vert(colour, &mut verticies, bottom_left_pos);
			push_vert(colour, &mut verticies, bottom_right_pos);
			push_vert(colour, &mut verticies, bottom_pos);

			let offset = ((pos.y * self.width) + pos.x) * 6;
			let [top, top_left, top_right, bottom_left, bottom_right, bottom] = [offset + 0, offset + 1, offset + 2, offset + 3, offset + 4, offset + 5];
			tris.extend([top, top_left, top_right]);
			tris.extend([top_right, top_left, bottom_right]);
			tris.extend([bottom_right, top_left, bottom_left]);
			tris.extend([bottom_left, bottom, bottom_right]);

			if let Some((_, _)) = self.sample_left(topo, pos).filter(|next_value| next_value.0 != elevation) {
				let [upper1, upper2] = [top_left, bottom_left];
				let [lower1, lower2] = [top_right - 6, bottom_right - 6];
				tris.extend([upper2, upper1, lower2]);
				tris.extend([lower2, upper1, lower1]);
			}
			if let Some((_, pos)) = self.sample_up_left(topo, pos).filter(|next_value| next_value.0 != elevation) {
				let [upper1, upper2] = [top, top_left];
				let [lower1, lower2] = [((pos.y * self.width) + pos.x) * 6 + 4, ((pos.y * self.width) + pos.x) * 6 + 5];
				tris.extend([upper2, upper1, lower2]);
				tris.extend([lower2, upper1, lower1]);
			}
			if let Some((_, pos)) = self.sample_up_right(topo, pos).filter(|next_value| next_value.0 != elevation) {
				let [upper1, upper2] = [top_right, top];
				let [lower1, lower2] = [((pos.y * self.width) + pos.x) * 6 + 5, ((pos.y * self.width) + pos.x) * 6 + 3];
				tris.extend([upper2, upper1, lower2]);
				tris.extend([lower2, upper1, lower1]);
			}
		}

		//assert_eq!(verticies.len(), vertex_count, "Incorrect vert heuristic");
		//assert_eq!(tris.len(), tri_count, "Incorrect tri heuristic");
		(verticies, tris)
	}

	pub fn update_hover(&self, mat: Mat4, normalised_mouse: Vec2) {
		let a = mat.inverse().project_point3(normalised_mouse.extend(1.));
		let b = mat.inverse().transform_vector3(Vec3::Z);
		info!(
			"{:?}",
			ray_triangle(
				a,
				b,
				[
					Vec2::new(-Map::APOTHEM, -Map::RADII / 2.).extend(0.),
					Vec2::new(Map::APOTHEM, -Map::RADII / 2.).extend(0.),
					Vec2::new(-Map::APOTHEM, Map::RADII / 2.).extend(0.)
				]
			)
		);
	}
}

pub fn ray_triangle(ray_origin: Vec3, ray_direction: Vec3, vertex: [Vec3; 3]) -> Option<Vec3> {
	let edge1 = vertex[1] - vertex[0];
	let edge2 = vertex[2] - vertex[0];

	let h = ray_direction.cross(edge2);
	let a = edge1.dot(h);
	// Check if the ray is parallel to the triangle
	if a.abs() < f32::EPSILON {
		return None;
	}
	let f = 1. / a;
	let s = ray_origin - vertex[0];
	let u = f * s.dot(h);
	if u < 0. || u > 1. {
		return None;
	}

	let q = s.cross(edge1);
	let v = f * ray_direction.dot(q);

	if v < 0. || u + v > 1. {
		return None;
	}

	// T is where the intersection is on the line
	let t = f * edge2.dot(q);
	if t > f32::EPSILON {
		let out_intersection_point = ray_origin + ray_direction * t;
		return Some(out_intersection_point);
	}
	return None;
}
