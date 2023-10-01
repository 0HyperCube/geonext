#version 300 es

#ifdef GL_ES
precision mediump float;
#endif

in vec3 primaryColour;
in vec3 secondaryColour;
in vec2 uv;

out vec4 FragColor;

float p = 0.4;

void main()
{
	
	float triangleWave = (2. * abs(uv.x / p - floor(uv.x / p + 0.5)) - 0.5) * 5.;
	FragColor = vec4(mix(primaryColour, secondaryColour, triangleWave), 1.-uv.y);
}
