#version 450

layout(push_constant) uniform PushConstants {
    float time;
    float res_x;
    float res_y;
} constants;

layout(location = 0) out vec4 out_color;

void main()
{
    float time = constants.time;
    // For shadertoy:
    // vec2 frag_coord = vec2(gl_FragCoord.x, constants.res_y - gl_FragCoord.y);
    vec2 frag_coord = gl_FragCoord.xy;
    vec2 resolution = vec2(constants.res_x, constants.res_y);

    out_color = vec4(frag_coord / resolution, 1.0, 0.0);
}
