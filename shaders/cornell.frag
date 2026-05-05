#version 450

layout(location = 0) in vec3 v_normal;
layout(location = 1) in vec3 v_world_pos;
layout(location = 2) in vec4 v_albedo;

layout(location = 0) out vec4 o_color;

void main() {
    vec3 n = normalize(v_normal);
    vec3 light_pos = vec3(0.0, 1.85, 0.0);
    vec3 to_light = light_pos - v_world_pos;
    float dist2 = max(dot(to_light, to_light), 1e-3);
    vec3 l = to_light / sqrt(dist2);
    float ndl = dot(n, l);
    float h = ndl * 0.5 + 0.5;
    float wrap = h * h;
    float falloff = 1.0 / (0.5 + 0.5 * dist2);
    vec3 ambient = v_albedo.rgb * 0.15;
    vec3 direct = v_albedo.rgb * wrap * falloff * 1.4;
    o_color = vec4(ambient + direct, 1.0);
}
