#version 300 es

#ifdef GL_ES
precision mediump float;
#endif

out vec4 FragColor;

in vec4 vertexColour;
in vec3 normal;

float near = 0.1; 
float far  = 100.0; 

float LinearDepth(float depth) {
	float z = depth * 2.0 - 1.0;
	return (2.0 * near * far) / (far + near - z * (far - near));
}

void main()
{
	float depth = LinearDepth(gl_FragCoord.z) / far;
	vec3 lightDir = normalize(vec3(1));
	FragColor = vertexColour * vec4(vec3(max(dot(normal, lightDir), 0.3)), 1);
}
