#version 450

layout(set = 0, binding = 0) uniform Globals {
    mat4 view_proj;
    mat4 model;
} u;

layout(location = 0) in vec3 in_pos;
layout(location = 1) in vec3 in_normal;
layout(location = 2) in vec2 in_uv;

layout(location = 0) out vec2 v_uv;
layout(location = 1) out vec3 v_normal;

void main() {
    vec4 world = u.model * vec4(in_pos, 1.0);
    gl_Position = u.view_proj * world;
    v_uv = in_uv;
    v_normal = normalize(mat3(u.model) * in_normal);
}
