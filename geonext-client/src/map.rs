use std::ops::Range;

use glam::{Vec2, Vec3};

#[derive(Debug, Default)]
pub struct Map {
	map: Vec<u8>,
	width: usize,
	height: usize,
	channels: usize,
	name_index: Vec<Range<usize>>,
	data_start: usize,
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
		self.width = u16::from_le_bytes([index.next(), index.next()]) as usize;
		self.height = u16::from_le_bytes([index.next(), index.next()]) as usize;
		self.channels = u16::from_le_bytes([index.next(), index.next()]) as usize;
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

		info!("data start {} Topo {}", self.data_start, self.sample_at(3, 50, 20));
	}
	pub fn get_name(&self, index: usize) -> &str {
		if let Some(name_index) = self.name_index.get(index) {
			// Note that python always encodes strings correctly
			unsafe { std::str::from_utf8_unchecked(&self.map[name_index.clone()]) }
		} else {
			"Water"
		}
	}
	pub fn sample_at(&self, channel: usize, x: usize, y: usize) -> u8 {
		unsafe { *self.map.get_unchecked(self.data_start + (x * self.height + y) * self.channels + channel) }
	}

	pub fn generate_terrain(&self) -> (Vec<f32>, Vec<usize>) {
		assert!(!self.map.is_empty(), "Map should be populated");

		let vertex_count = (self.width * self.height) * 6 * 6;
		let tri_count = (self.width * self.height) * 4 * 3;
		let mut verticies = Vec::with_capacity(vertex_count);
		let mut tris = Vec::with_capacity(tri_count);

		let push_vert = |verticies: &mut Vec<f32>, pos: Vec3| {
			verticies.extend(pos.to_array());
			verticies.extend([0.5_f32; 3]);
		};

		let channel = 3;

		for (x, y) in (0..self.height).flat_map(|y| (0..self.width).map(move |x| (x, y))) {
			let centre = Vec2::new(((x * 2 + 1 - (y % 2)) as f32 - 1.) * Map::APOTHEM, y as f32 * (Map::RADII * 3.) / 2.);
			let value = self.sample_at(channel, x, y);
			let height = (if value > 240 { -0.01 } else { value as f32 / 255. }) * 5.;
			let top_pos = (centre + Vec2::new(0., -Map::RADII)).extend(height);
			let top_left_pos = (centre + Vec2::new(-Map::APOTHEM, -Map::RADII / 2.)).extend(height);
			let top_right_pos = (centre + Vec2::new(Map::APOTHEM, -Map::RADII / 2.)).extend(height);
			let bottom_left_pos = (centre + Vec2::new(-Map::APOTHEM, Map::RADII / 2.)).extend(height);
			let bottom_right_pos = (centre + Vec2::new(Map::APOTHEM, Map::RADII / 2.)).extend(height);
			let bottom_pos = (centre + Vec2::new(0., Map::RADII)).extend(height);

			push_vert(&mut verticies, top_pos);
			push_vert(&mut verticies, top_left_pos);
			push_vert(&mut verticies, top_right_pos);
			push_vert(&mut verticies, bottom_left_pos);
			push_vert(&mut verticies, bottom_right_pos);
			push_vert(&mut verticies, bottom_pos);

			let offset = ((y * self.width) + x) * 6;
			let [top, top_left, top_right, bottom_left, bottom_right, bottom] = [offset + 0, offset + 1, offset + 2, offset + 3, offset + 4, offset + 5];
			tris.extend([top, top_left, top_right]);
			tris.extend([top_right, top_left, bottom_right]);
			tris.extend([bottom_right, top_left, bottom_left]);
			tris.extend([bottom_left, bottom, bottom_right]);

			if x > 0 {
				let next_value = self.sample_at(channel, x - 1, y);
				if next_value != value {
					let [upper1, upper2] = [top_left, bottom_left];
					let [lower1, lower2] = [top_right - 6, bottom_right - 6];
					tris.extend([upper2, upper1, lower2]);
					tris.extend([lower2, upper1, lower1]);
				}
			}
			if y > 0 && x >= y % 2 {
				let x = x - (y % 2);
				let y = y - 1;
				let next_value = self.sample_at(channel, x, y);
				if next_value != value {
					let [upper1, upper2] = [top, top_left];
					let [lower1, lower2] = [((y * self.width) + x) * 6 + 4, ((y * self.width) + x) * 6 + 5];
					tris.extend([upper2, upper1, lower2]);
					tris.extend([lower2, upper1, lower1]);
				}
			}
			if y > 0 && x < self.width - 1 + y % 2 {
				let x = x + 1 - (y % 2);
				let y = y - 1;
				let next_value = self.sample_at(channel, x, y);
				if next_value != value {
					let [upper1, upper2] = [top_right, top];
					let [lower1, lower2] = [((y * self.width) + x) * 6 + 5, ((y * self.width) + x) * 6 + 3];
					tris.extend([upper2, upper1, lower2]);
					tris.extend([lower2, upper1, lower1]);
				}
			}
		}

		//assert_eq!(verticies.len(), vertex_count, "Incorrect vert heuristic");
		//assert_eq!(tris.len(), tri_count, "Incorrect tri heuristic");
		(verticies, tris)
	}
}
