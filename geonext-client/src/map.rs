use geonext_shared::{
	map_loader::{HeightMap, HexCoord},
	territories::{CountryId, Territories},
};
use glam::{Mat4, UVec2, Vec2, Vec3};

use crate::EventType;

#[derive(Debug, Default)]
pub struct Map {
	pub height_map: HeightMap,
	pub borders: Territories,
	hovered: UVec2,
	pub updated: bool,
}

impl Map {
	pub fn load(&mut self, map: Vec<u8>) {
		self.height_map.load(map);
	}

	pub fn update_hover(&mut self, projection: Mat4, view: Mat4, normalised_mouse: Vec2) {
		let px_nds = (normalised_mouse - 0.5) * Vec2::new(2., -2.);
		//let px_nds = Vec2::ZERO;
		let point_nds = px_nds.extend(-1.);
		let mut dir_eye = projection.inverse() * point_nds.extend(1.);
		dir_eye.w = 0.;
		let ray_direction = (view.inverse() * dir_eye).truncate().normalize();
		let ray_origin = view.inverse().w_axis.truncate();

		let hovered = (0..self.height_map.width as i32)
			.flat_map(|x| (0..self.height_map.height as i32).map(move |y| (x, y)))
			.find(|&(x, y)| HexCoord::from_offset(x, y).intersect_ray(0., ray_origin, ray_direction).is_some());
		let Some((x, y)) = hovered else {
			return;
		};

		self.hovered = UVec2::new(x as u32, y as u32);
	}

	pub fn hovered_name(&self) -> &str {
		self.borders.get_name(self.borders.country_id(self.hovered))
	}
	pub fn hovered_country(&self) -> CountryId {
		self.borders.country_id(self.hovered)
	}
}

pub fn ray_ground(ray_origin: Vec3, ray_direction: Vec3) -> Option<Vec3> {
	let normal = Vec3::Z;
	let denom = normal.dot(ray_direction);
	if denom.abs() > f32::EPSILON * 100. {
		let t = (-ray_origin).dot(normal) / denom;
		return Some(ray_origin + ray_direction * t);
	}
	None
}
