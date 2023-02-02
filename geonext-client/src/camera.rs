use glam::{IVec2, Mat4, Vec2, Vec3};

use crate::terrain::Terrain;

const CAMERA_CLEARENCE: f32 = 10.;
const OFFSET: Vec2 = Vec2::new(0., -20.);

/// Controls the movement of the camera over the map
#[derive(Debug)]
pub struct Camera {
	pub position: Vec2,
	pub zoom: f32,
	pub velocity: Vec2,
}

impl Default for Camera {
	fn default() -> Self {
		Self {
			position: Vec2::ZERO,
			zoom: 1.,
			velocity: Vec2::ZERO,
		}
	}
}

impl Camera {
	/// Move the camera by a coordinate in screen space (not very pracise)
	pub fn move_by(&mut self, screen: Vec2) {
		self.position += (screen * self.zoom) / 10.
	}

	/// Construct a mat4 based on the camera's position and zoom
	pub fn to_matrix(&self, terrain: &Terrain) -> Mat4 {
		let height = terrain.sample_at(self.position + OFFSET);
		let eye = self.position.extend(height + CAMERA_CLEARENCE + self.zoom * 100.);
		let centre = (self.position + OFFSET).extend(height);
		Mat4::from_scale(Vec3::new(-1., 1., 1.)) * Mat4::from_rotation_z(core::f32::consts::PI) * Mat4::look_at_rh(eye, centre, Vec3::new(0., 1., 0.))
	}

	/// Handles a mouse drag
	pub fn mouse_move(&mut self, delta: IVec2) {
		self.move_by(-delta.as_vec2());
	}
}
