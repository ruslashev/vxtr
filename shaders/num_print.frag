#version 450

layout(push_constant) uniform PushConstants {
    float time;
    float res_x;
    float res_y;
} constants;

layout(location = 0) out vec4 out_color;

// ---- 8< -------- 8< -------- 8< -------- 8< ----
// GLSL Number printing: Originally by @P_Malin https://www.shadertoy.com/view/4sBSWW
// Creative Commons CC0 1.0 Universal (CC-0)
vec3 PrintValue(vec3 in_color, vec3 text_color, vec2 frag_coord, vec2 coords,
        float scale, float value, int max_digits, int decimal_places)
{
    const int minus = 1792;
    const int fractdot = 2;
    const int[] digits = int[](480599, 139810, 476951, 476999, 350020, 464711, 464727, 476228, 481111, 481095);
    const vec2 font_size = vec2(4.0, 5.0);

    coords = frag_coord - coords;
    coords = coords / (font_size * scale);

    if (coords.y < 0.0 || coords.y >= 1.0)
        return in_color;

    bool is_neg = value < 0.0;
    value = abs(value);
    int num_digits = max(int(floor(log2(value) / log2(10.0))), 0);
    int digit_idx = max_digits - int(floor(coords.x));
    int char_bin = 0;

    if (digit_idx > -decimal_places - 1.01) {
        if (digit_idx > num_digits) {
            if (is_neg && digit_idx < num_digits + 2)
                char_bin = minus;
        } else {
            if (digit_idx == -1) {
                if (decimal_places > 0)
                    char_bin = fractdot;
            } else {
                float rval = value;
                if (digit_idx < 0) { rval = fract(value); digit_idx += 1; }
                char_bin = digits[int(floor(rval / pow(10.0, digit_idx))) % 10];
            }
        }
    }

    float p = floor(fract(coords.x) * 4.0) + floor(coords.y * 5.0) * 4.0;
    float weight = floor(mod(char_bin / pow(2.0, p), 2.0));

    return mix(in_color, text_color, weight);
}
// ---- 8< -------- 8< -------- 8< -------- 8< ----

void main()
{
    float time = constants.time;
    vec2 frag_coord = vec2(gl_FragCoord.x, constants.res_y - gl_FragCoord.y);
    vec2 resolution = vec2(constants.res_x, constants.res_y);

    vec3 color = vec3(0.0);

    color = PrintValue(color, vec3(0.0, 1.0, 1.0), frag_coord, vec2(5.0, 5.0), 2.0, -time * 10, 2, 5);

    out_color = vec4(color, 1.0);
}
