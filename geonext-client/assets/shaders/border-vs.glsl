#version 300 es
layout (location = 0) in vec3 aPos;
layout (location = 1) in vec3 aPrimaryColour;
layout (location = 2) in vec3 aSecondaryColour;
layout (location = 3) in vec2 aUv;


out vec3 primaryColour;
out vec3 secondaryColour;
out vec2 uv;

uniform mat4 model;
uniform mat4 view;
uniform mat4 projection;

void main()
{
	gl_Position = projection * view * model * vec4(aPos, 1.0);
	primaryColour = aPrimaryColour;
	secondaryColour = aSecondaryColour;
	uv = aUv;
}
