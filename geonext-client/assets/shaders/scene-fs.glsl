#version 300 es

#ifdef GL_ES
precision mediump float;
#endif

out vec4 FragColor;

in vec4 vertexColour;

void main()
{
	FragColor =  vertexColour;
}
