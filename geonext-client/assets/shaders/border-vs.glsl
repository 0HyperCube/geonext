#version 300 es
layout (location = 0) in vec3 aPos;
layout (location = 1) in vec3 aPrimaryColour;
layout (location = 2) in vec3 aSecondaryColour;
layout (location = 3) in vec2 aUv;
layout (location = 4) in uint avalue;


out vec3 primaryColour;
out vec3 secondaryColour;
out vec2 uv;

uniform mat4 model;
uniform mat4 view;
uniform mat4 projection;
uniform uint select;

void main()
{
	gl_Position = projection * view * model * vec4(aPos, 1.0) - vec4(0., 0.0, 0.0002, 0.);
	primaryColour = aPrimaryColour;
	secondaryColour = aSecondaryColour;
	if (select == avalue) {
		primaryColour = vec3(1);
		secondaryColour = vec3(1);
	}
	uv = aUv;
}
