#version 300 es

#ifdef GL_ES
precision mediump float;
#endif

out vec4 FragColor;

in vec4 vertexColour;

float near = 0.1; 
float far  = 100.0; 

float LinearDepth(float depth) {
	float z = depth * 2.0 - 1.0;
	return (2.0 * near * far) / (far + near - z * (far - near));
}

void main()
{
	float depth = LinearDepth(gl_FragCoord.z) / far;
	FragColor = vertexColour;
}
