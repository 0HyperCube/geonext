use std::{collections::HashMap, rc::Rc};

use fontdue::{Font, Metrics};
use glam::{ UVec2, Vec2};
use glow::HasContext;

use crate::{Assets, ErrorKind};

use super::atlas::Atlas;

/// Contains a cache of the fonts that have already been parsed (reducing the need to parse the font many times)
#[derive(Debug, Default)]
struct ParsedFonts(HashMap<&'static str, Font>);

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

	/// Returns a parsed font, parsing it if it is not yet in the cache
	pub fn add_font(&mut self, assets: &Assets, font_name: &'static str) {
		self.parsed_fonts
			.0
			.entry(font_name)
			.or_insert_with(|| fontdue::Font::from_bytes(&assets.get(font_name) as &[u8], fontdue::FontSettings::default()).expect("Failed to parse font"));
	}

	pub fn init(&mut self) -> Result<(), ErrorKind> {
		self.atlas = Atlas::new(UVec2::splat(TEXTURE_SIZE as u32));
		unsafe {
			// Disable 4 byte alignment
			self.context.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);

			self.texture = Some(self.context.create_texture().map_err(ErrorKind::CreateTexture)?);
			self.context.bind_texture(glow::TEXTURE_2D, self.texture);

			let pixels = Some(&core::slice::from_raw_parts(5 as *const u8, (TEXTURE_SIZE * TEXTURE_SIZE) as usize) as &[u8]);

			self.context
				.tex_image_2d(glow::TEXTURE_2D, 0, glow::R8 as i32, TEXTURE_SIZE, TEXTURE_SIZE, 0, glow::RED, glow::UNSIGNED_BYTE, pixels);
			self.context.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
			self.context.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);
			self.context.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR as i32);
			self.context.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
			info!("Tex image 2d");

			info!("Initalised font");
		}
		Ok(())
	}

	fn load_glyph<'a>(glyphs: &'a mut HashMap<u16, GlyphCache>, atlas: &mut Atlas, context: &Rc<glow::Context>, index: u16, font: &Font) -> &'a GlyphCache {
		glyphs.entry(index).or_insert_with(|| {
			// Rasterize and get the layout metrics for the letter 'g' at 17px.
			let (metrics, bitmap) = font.rasterize_indexed(index, 50.);
			let pos = atlas
				.allocate_rect(UVec2::new(metrics.width as u32, metrics.height as u32))
				.expect("Too many glyphs (todo: new texture or something)");
			unsafe {
				context.tex_sub_image_2d(
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
			GlyphCache { pos, metrics }
		})
	}
	pub fn render_glyphs(&mut self, text: &str, font_name: &'static str, mut pos: Vec2, vbo: <glow::Context as glow::HasContext>::Buffer) {
		let font = self.parsed_fonts.0.get(font_name).expect("Tried to use unloaded font");

		unsafe {
			self.context.bind_texture(glow::TEXTURE_2D, Some(self.texture.unwrap()));
			info!("Bound texture");
		}

		for c in text.chars() {
			let index = font.lookup_glyph_index(c);
			let glyph = Self::load_glyph(&mut self.glyphs, &mut self.atlas, &self.context, index, font);
			let render_pos = Vec2::new(pos.x + glyph.metrics.xmin as f32, pos.y + glyph.metrics.ymin as f32);
			let size = Vec2::new(glyph.metrics.width as f32, glyph.metrics.height as f32);

			let texture_size = Vec2::splat(TEXTURE_SIZE as f32);
			let uv_min = glyph.pos.as_vec2() / texture_size;
			let uv_max = (glyph.pos.as_vec2() + size) / texture_size;

			info!("render {render_pos}, size {size}");
			let verticies = [
				((render_pos.x, render_pos.y + size.y), (uv_min.x, uv_min.y)),
				((render_pos.x, render_pos.y), (uv_min.x, uv_max.y)),
				((render_pos.x + size.x, render_pos.y), (uv_max.x, uv_max.y)),
				((render_pos.x, render_pos.y + size.y), (uv_min.x, uv_min.y)),
				((render_pos.x + size.x, render_pos.y), (uv_max.x, uv_max.y)),
				((render_pos.x + size.x, render_pos.y + size.y), (uv_max.x, uv_min.y)),
			];
			unsafe {
				let (_, src_data, _) = verticies.align_to();
				info!("{verticies:?} src {src_data:?}");
				self.context.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
				self.context.buffer_sub_data_u8_slice(glow::ARRAY_BUFFER, 0, src_data);
				self.context.bind_buffer(glow::ARRAY_BUFFER, None);
				// render quad
				self.context.draw_arrays(glow::TRIANGLES, 0, 6);
			}
			// now advance cursors for next glyph (note that advance is number of 1/64 pixels)
			pos += Vec2::new(glyph.metrics.advance_width, glyph.metrics.advance_height);
		}
		unsafe {
			self.context.bind_texture(glow::TEXTURE_2D, None);
			info!("unbound texture");
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
