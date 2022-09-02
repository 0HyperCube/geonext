use std::rc::Rc;

use glam::{Mat4, Vec3, Vec4};
use glow::HasContext;

use crate::ErrorKind;

/// Wrapper around a glow shader, cleaning up on drop
pub struct Shader {
	context: Rc<glow::Context>,
	shader: <glow::Context as HasContext>::Shader,
}

impl Shader {
	/// Construct a new shader
	pub fn new(context: Rc<glow::Context>, shader_type: u32, source: &str) -> Result<Self, ErrorKind> {
		let shader = unsafe {
			let shader = context.create_shader(shader_type).map_err(ErrorKind::ShaderCompileError)?;
			context.shader_source(shader, source);
			context.compile_shader(shader);
			shader
		};

		let success = unsafe { context.get_shader_compile_status(shader) };
		if !success {
			let error = unsafe { context.get_shader_info_log(shader) };

			let name = match shader_type {
				glow::VERTEX_SHADER => "Vertex stage",
				glow::FRAGMENT_SHADER => "Fragment stage",
				_ => "Shader stage",
			};

			return Err(ErrorKind::ShaderCompileError(format!("Shader error in {}: {}", name, error)));
		}

		Ok(Self { context, shader })
	}
}

impl Drop for Shader {
	fn drop(&mut self) {
		unsafe {
			self.context.delete_shader(self.shader);
		}
	}
}

/// Wrapper around a glow program, providing setters and cleaning up on drop
pub struct Program {
	context: Rc<glow::Context>,
	program: <glow::Context as HasContext>::Program,
}

#[allow(dead_code)]
impl Program {
	/// Construct a new shader program
	pub fn new(context: Rc<glow::Context>, shaders: &[Shader], attribute_locations: &[&str]) -> Result<Self, ErrorKind> {
		let program = unsafe { context.create_program().map_err(ErrorKind::ProgramLinkError)? };
		unsafe {
			for shader in shaders {
				context.attach_shader(program, shader.shader);
			}
			for (index, name) in attribute_locations.iter().enumerate() {
				context.bind_attrib_location(program, index as u32, name);
			}
			context.link_program(program);
			if !context.get_program_link_status(program) {
				return Err(ErrorKind::ProgramLinkError(context.get_program_info_log(program)));
			}

			// Detach shaders
			for shader in shaders {
				context.detach_shader(program, shader.shader);
			}
		}
		Ok(Self { context, program })
	}

	/// Use this program for the next draw calls
	pub fn bind(&self) {
		unsafe { self.context.use_program(Some(self.program)) };
	}

	/// Stop using this shader program
	pub fn unbind(&self) {
		unsafe { self.context.use_program(None) };
	}

	/// Set a boolean uniform
	pub fn set_bool(&self, name: &str, value: bool) {
		unsafe {
			let location = self.context.get_uniform_location(self.program, name).expect(&format!("failed to find location for bool '{name}'"));
			self.context.uniform_1_i32(Some(&location), value as i32);
		}
	}

	/// Set an i32 uniform
	pub fn set_int(&self, name: &str, value: i32) {
		unsafe {
			let location = self.context.get_uniform_location(self.program, name).expect(&format!("failed to find location for i32 '{name}'"));
			self.context.uniform_1_i32(Some(&location), value);
		}
	}

	/// Set an f32 uniform
	pub fn set_float(&self, name: &str, value: f32) {
		unsafe {
			let location = self.context.get_uniform_location(self.program, name).expect(&format!("failed to find location for f32 '{name}'"));
			self.context.uniform_1_f32(Some(&location), value);
		}
	}

	/// Set a vec3 uniform
	pub fn set_vec3(&self, name: &str, value: Vec3) {
		unsafe {
			let location = self.context.get_uniform_location(self.program, name).expect(&format!("failed to find location for vec3 '{name}'"));
			self.context.uniform_3_f32(Some(&location), value.x, value.y, value.z);
		}
	}

	/// Set a vec4 uniform
	pub fn set_vec4(&self, name: &str, value: Vec4) {
		unsafe {
			let location = self.context.get_uniform_location(self.program, name).expect(&format!("failed to find location for vec4 '{name}'"));
			self.context.uniform_4_f32(Some(&location), value.x, value.y, value.z, value.w);
		}
	}

	/// Set a mat4 uniform
	pub fn set_mat4(&self, name: &str, value: Mat4) {
		unsafe {
			let location = self.context.get_uniform_location(self.program, name).expect(&format!("failed to find location for mat4 '{name}'"));
			self.context.uniform_matrix_4_f32_slice(Some(&location), false, &value.to_cols_array());
		}
	}
}

impl Drop for Program {
	fn drop(&mut self) {
		unsafe { self.context.delete_program(self.program) };
	}
}
