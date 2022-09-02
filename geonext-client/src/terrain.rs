use glam::{UVec2, Vec2};
use image::{DynamicImage, GenericImageView};
use random_fast_rng::{FastRng, Random};

/// Draws the world map
#[derive(Default)]
pub struct Terrain {
	terrain: DynamicImage,
	pub size: UVec2,
}

impl core::fmt::Debug for Terrain {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("terrain").field("size", &self.size).finish()
	}
}

const HEIGHT_CHANNEL: usize = 1;

impl Terrain {
	/// Loads the terrain texture from memory
	pub fn load(&mut self, img: &[u8]) {
		self.terrain = image::load_from_memory_with_format(img, image::ImageFormat::Jpeg).expect("Failed to load terrain image");
		//let terrain = terrain.resize_exact(2, 2, image::imageops::FilterType::Triangle);
		self.size = UVec2::new(self.terrain.width(), self.terrain.height());
	}

	/// Convert a pixel value on the texture to a height
	#[inline]
	pub fn value_to_height(value: u8) -> f32 {
		(if value > 240 { -0.01 } else { value as f32 / 255. }) * 5.
	}

	/// Generates the terrain mesh based on the texture that has been loaded
	pub fn generate_terrain(&self) -> (Vec<f32>, Vec<usize>) {
		assert_ne!(self.size, UVec2::ZERO, "Terrain should be initalised");

		let capacity = (self.size.x * self.size.y * 2 * 3) as usize;
		let mut vertices = Vec::with_capacity(capacity);

		let mut rng = FastRng::seed(42, 42 * 42);
		for (x, y, pixel) in self.terrain.pixels() {
			vertices.push(scatter(x as f32, &mut rng));
			vertices.push(scatter(y as f32, &mut rng));

			vertices.push(Self::value_to_height(pixel[HEIGHT_CHANNEL]));

			vertices.push(0.5);
			vertices.push(0.5);
			vertices.push(0.5);
		}
		assert_eq!(vertices.len(), capacity);

		let capacity = ((self.size.x - 1) * (self.size.y - 1) * 2 * 3) as usize;
		let mut tris = Vec::with_capacity(capacity);
		for (x, y) in (0..self.size.x - 1).flat_map(|x| (0..self.size.y - 1).map(move |y| (x, y))) {
			let top_left = x + y * self.size.x;
			let top_right = top_left + 1;
			let bottom_left = top_left + self.size.x;
			let bottom_right = bottom_left + 1;
			tris.push(bottom_left as usize);
			tris.push(top_right as usize);
			tris.push(top_left as usize);

			tris.push(bottom_left as usize);
			tris.push(top_right as usize);
			tris.push(bottom_right as usize);
		}

		assert_eq!(tris.len(), capacity);

		(vertices, tris)
	}
	pub fn sample_at(&self, pos: Vec2) -> f32 {
		let floored = pos.floor();

		Self::value_to_height(self.terrain.get_pixel((floored.x as u32).clamp(0, self.size.x - 1), (floored.y as u32).clamp(0, self.size.y - 1))[HEIGHT_CHANNEL])
	}
}

/// Generate a random f32 between 0 and 1 with FastRng
fn rand_float(rng: &mut FastRng) -> f32 {
	const FLOAT_NORM: f32 = 1.0f32 / (1u64 << 32) as f32;
	rng.get_u32() as f32 * FLOAT_NORM
}

/// Offset a position by a random amount
fn scatter(position: f32, rng: &mut FastRng) -> f32 {
	position + rand_float(rng) - 0.5
}
