use std::{collections::HashMap, rc::Rc};

use fontdue::{Font, Metrics};
use glam::UVec2;
use glow::HasContext;

use crate::{Assets, ErrorKind};

use super::atlas::Atlas;

/// Contains a cache of the fonts that have already been parsed (reducing the need to parse the font many times)
#[derive(Debug, Default)]
struct ParsedFonts(HashMap<&'static str, Font>);

impl ParsedFonts {
	/// Returns a parsed font, parsing it if it is not yet in the cache
	fn get_or_parse(&mut self, assets: &Assets, font_name: &'static str) -> &Font {
		if !self.0.contains_key(font_name) {
			self.0.insert(
				font_name,
				fontdue::Font::from_bytes(&assets.get(font_name) as &[u8], fontdue::FontSettings::default()).expect("Failed to parse font"),
			);
		}
		self.0.get(font_name).unwrap()
	}
}

#[derive(Debug, Default)]
struct GlyphCache {
	pos: UVec2,
	metrics: Metrics,
}

/// Stores peices of text rendering between frames to increase performance
#[derive(Debug)]
pub struct FontCache {
	context: Rc<glow::Context>,
	texture: Option<<glow::Context as HasContext>::Texture>,
	parsed_fonts: ParsedFonts,
	glyphs: HashMap<u16, GlyphCache>,
	atlas: Atlas,
}
const TEXTURE_SIZE: i32 = 512;
impl FontCache {
	pub fn new(context: Rc<glow::Context>) -> Self {
		Self {
			context,
			texture: Default::default(),
			parsed_fonts: Default::default(),
			glyphs: Default::default(),
			atlas: Default::default(),
		}
	}

	pub fn init(&mut self) -> Result<(), ErrorKind> {
		self.atlas = Atlas::new(UVec2::splat(TEXTURE_SIZE as u32));
		unsafe {
			// Disable 4 byte alignment
			self.context.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);

			self.context.active_texture(glow::TEXTURE0);
			self.texture = Some(self.context.create_texture().map_err(ErrorKind::CreateTexture)?);
			self.context.bind_texture(glow::TEXTURE_2D, self.texture);
			self.context.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);

			let pixels = Some(&core::slice::from_raw_parts(5 as *const u8, (TEXTURE_SIZE * TEXTURE_SIZE) as usize) as &[u8]);

			self.context
				.tex_image_2d(glow::TEXTURE_2D, 0, glow::R8 as i32, TEXTURE_SIZE, TEXTURE_SIZE, 0, glow::RED, glow::UNSIGNED_BYTE, pixels);
			info!("Tex image 2d");

			info!("Initalised font");
		}
		Ok(())
	}

	pub fn load_glyphs(&mut self, text: &str, assets: &Assets, font_name: &'static str) {
		let font = self.parsed_fonts.get_or_parse(assets, font_name);

		unsafe {
			self.context.bind_texture(glow::TEXTURE_2D, Some(self.texture.unwrap()));
			info!("Bound texture");
		}

		for c in text.chars() {
			let index = font.lookup_glyph_index(c);
			if !self.glyphs.contains_key(&index) {
				// Rasterize and get the layout metrics for the letter 'g' at 17px.
				let (metrics, bitmap) = font.rasterize_indexed(index, 17.0);
				let pos = self
					.atlas
					.allocate_rect(UVec2::new(metrics.width as u32, metrics.height as u32))
					.expect("Too many glyphs (todo: new texture or something)");
				unsafe {
					self.context.tex_sub_image_2d(
						glow::TEXTURE_2D,
						0,
						pos.x as i32,
						pos.y as i32,
						metrics.width as i32,
						metrics.height as i32,
						glow::RED,
						glow::UNSIGNED_BYTE,
						glow::PixelUnpackData::Slice(&bitmap),
					);
					info!("Finished subimage");
				}
			}
		}
	}
}

impl Drop for FontCache {
	fn drop(&mut self) {
		if let Some(texture) = self.texture {
			unsafe { self.context.delete_texture(texture) }
		}
	}
}
