#version 450

layout(set = 0, binding = 1) uniform sampler2D u_albedo;

layout(location = 0) in vec2 v_uv;
layout(location = 1) in vec3 v_normal;

layout(location = 0) out vec4 o_color;

void main() {
    vec3 albedo = texture(u_albedo, v_uv).rgb;
    float shade = 0.5 + 0.5 * clamp(dot(normalize(v_normal), normalize(vec3(0.4, 0.8, 0.5))), 0.0, 1.0);
    o_color = vec4(albedo * shade, 1.0);
}
