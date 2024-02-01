use glam::{UVec2, Vec3};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct CountryId(u8);

impl CountryId {
	pub const SEA: Self = CountryId(254);
	pub fn colours(&self) -> [Vec3; 2] {
		const COLOURS: [(u8, u8, u8); 12] = [
			(128, 0, 0),
			(255, 127, 80),
			(255, 165, 0),
			(255, 215, 0),
			(128, 128, 0),
			(0, 255, 0),
			(0, 128, 128),
			(100, 149, 237),
			(0, 191, 255),
			(65, 105, 225),
			(147, 112, 219),
			(255, 0, 255),
		];
		let (r, g, b) = COLOURS[self.0 as usize % COLOURS.len()];
		let primary = Vec3::new(r as f32, g as f32, b as f32) / 255.;
		let secondary = Vec3::new(0., 0., 1.);
		[primary, secondary]
	}
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Territories {
	width: u32,
	hexes: Vec<CountryId>,
	country_names: Vec<String>,
}
impl Territories {
	pub fn get_name(&self, country: CountryId) -> &str {
		if let Some(name) = self.country_names.get(country.0 as usize) {
			&name
		} else {
			"Water"
		}
	}
	pub fn country_id(&self, pos: UVec2) -> CountryId {
		unsafe { *self.hexes.get_unchecked((pos.y * self.width + pos.x) as usize) }
	}
	pub fn height(&self) -> u32 {
		(self.hexes.len() as u32).checked_div(self.width).unwrap_or_default()
	}
	pub fn width(&self) -> u32 {
		self.width
	}
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TerritoriesRLE {
	width: u32,
	hexes: Vec<(u16, CountryId)>,
	country_names: Vec<String>,
}

impl Territories {
	pub fn to_rle(&self) -> TerritoriesRLE {
		let mut hexes: Vec<(u16, CountryId)> = Vec::new();
		for &item in &self.hexes {
			let Some(last) = hexes.last_mut().filter(|last| last.1 == item) else {
				hexes.push((1, item));
				continue;
			};
			last.0 += 1;
		}
		TerritoriesRLE {
			width: self.width,
			hexes,
			country_names: self.country_names.clone(),
		}
	}
	pub fn from_rle(rle: &TerritoriesRLE) -> Self {
		Self {
			width: rle.width,
			hexes: rle.hexes.iter().flat_map(|(count, val)| core::iter::repeat(*val).take(*count as usize)).collect(),
			country_names: rle.country_names.clone(),
		}
	}
}

#[test]
fn load_territories() {
	let x: Territories = bincode::deserialize(include_bytes!("./../../assets/starting_game_map")).unwrap();
	let rle = x.to_rle();
	let y = Territories::from_rle(&rle);
	assert_eq!(x, y);
	println!("{:?}", x.width);
}
