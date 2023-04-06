#version 300 es
layout (location = 0) in vec2 vertex; // [pos.x, pos.y]
layout (location = 1) in vec4 instancePos; // [pos.x, pos.y, size.x, size.y]
layout (location = 2) in vec4 instanceUv; // [pos.x, pos.y, size.x, size.y]
out vec2 TexCoords;

uniform mat4 projection;

void main() {
	gl_Position = projection *  vec4(instancePos.xy + vertex * instancePos.zw, 0., 1.);
	TexCoords = instanceUv.xy + vertex * instanceUv.zw;
}
