#version 300 es
layout (location = 0) in vec3 aPos;
layout (location = 1) in vec3 aNormal;
layout (location = 2) in vec3 aColour;
layout (location = 3) in vec3 aOffset;

out vec4 vertexColour;
out vec3 normal;

uniform vec4 addColour;

uniform mat4 model;
uniform mat4 view;
uniform mat4 projection;

void main()
{
	gl_Position = projection * view * model * vec4(aPos+aOffset, 1.0);
	// A test colour to preview heights
	vec4 height = vec4(vec3(aPos.z / 10.), 1.0);
	vertexColour = vec4(aColour, 1.) + addColour;
	normal = aNormal;
}
