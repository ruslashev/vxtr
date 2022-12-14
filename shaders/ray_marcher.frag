#version 450

layout(push_constant) uniform PushConstants {
    float time;
    float res_x;
    float res_y;
} constants;

layout(location = 0) out vec4 out_color;

float sd_vert_plane(vec3 p, float o)
{
    return p.y - o;
}

float sd_sphere(vec3 p, vec3 o, float r)
{
    return distance(p, o) - r;
}

float map(vec3 p)
{
    float s1 = sd_sphere(p, vec3(-1,  0, -5), 1.);
    float s2 = sd_sphere(p, vec3( 2,  0, -3), 1.);
    float s3 = sd_sphere(p, vec3(-2,  0, -2), 1.);
    float pl = sd_vert_plane(p, -1.);

    float d = s1;

    d = min(d, s2);
    d = min(d, s3);
    d = min(d, pl);

    return d;
}

/// Calculate the normal by taking the central differences on the distance field
vec3 calc_normal(in vec3 p)
{
    vec2 e = vec2(1.0, -1.0) * 0.0005;
    return normalize(
        e.xyy * map(p + e.xyy) +
        e.yyx * map(p + e.yyx) +
        e.yxy * map(p + e.yxy) +
        e.xxx * map(p + e.xxx));
}

vec3 diffuse(vec3 p)
{
    vec3 normal = calc_normal(p);
    vec3 light = vec3(0, 2, 0);
    float light_intensity = 5.0;

    // Calculate diffuse lighting by taking the dot product of
    // the light direction (light-p) and the normal
    float dif = clamp(dot(normal, normalize(light - p)), 0., 1.);

    // Multiply by light intensity and divide by the square
    // of the distance to the light
    dif *= light_intensity / dot(light - p, light - p);

    return vec3(dif, dif, dif);
}

void main()
{
    float time = constants.time;
    vec2 frag_coord = vec2(gl_FragCoord.x, constants.res_y - gl_FragCoord.y);
    vec2 resolution = vec2(constants.res_x, constants.res_y);

    vec2 plane = (frag_coord.xy - resolution.xy * 0.5) / resolution.y;

    vec3 ro = vec3(0, 0, 1);
    vec3 rd = normalize(vec3(plane, 0.) - ro);

    // March the distance field until a surface is hit
    float h, t = 1.;
    int i;
    bool hit;
    for (i = 0; i < 64; i++) {
        h = map(ro + rd * t);
        t += h;
        if (h < 0.01) {
            hit = true;
            break;
        }
    }

    if (hit) {
        vec3 p = ro + rd * t;

        /* out_color = vec4(diffuse(p), 1.0); */
        float n = i / 64.0;
        out_color = vec4(n, n, n, 1.0);
    } else {
        out_color = vec4(0, 0, 0, 1);
    }
}
