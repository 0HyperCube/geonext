#version 300 es
layout (location = 0) in vec3 aPos;
layout (location = 1) in vec3 aColour;

out vec4 vertexColour;

uniform vec4 addColour;

uniform mat4 model;
uniform mat4 view;
uniform mat4 projection;

void main()
{
	gl_Position = projection * view * model * vec4(aPos, 1.0);
	// A test colour to preview heights
	vertexColour = vec4(vec3(aPos.z / 10.), 1.0) + addColour;
}
