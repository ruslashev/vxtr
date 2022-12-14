#version 450

layout(push_constant) uniform PushConstants {
    float time;
    float res_x;
    float res_y;
} constants;

layout(location = 0) out vec4 out_color;

const bool USE_BRANCHLESS_DDA = true;
const int MAX_RAY_STEPS = 64;

float sdSphere(vec3 p, float d)
{
    return length(p) - d;
}

float sdBox(vec3 p, vec3 b)
{
    vec3 d = abs(p) - b;
    return min(max(d.x, max(d.y, d.z)), 0.0) + length(max(d, 0.0));
}

bool getVoxel(vec3 p)
{
    p = p + vec3(0.5);
    float d = min(max(-sdSphere(p, 7.5), sdBox(p, vec3(6.0))), -sdSphere(p, 25.0));
    return d < 0.0;
}

vec2 rotate2d(vec2 v, float a)
{
    float sinA = sin(a);
    float cosA = cos(a);
    return vec2(v.x * cosA - v.y * sinA, v.y * cosA + v.x * sinA);
}

void main()
{
    float time = constants.time;
    // For shadertoy:
    // vec2 frag_coord = vec2(gl_FragCoord.x, constants.res_y - gl_FragCoord.y);
    vec2 frag_coord = gl_FragCoord.xy;
    vec2 resolution = vec2(constants.res_x, constants.res_y);

    vec2 screenPos = (frag_coord.xy / resolution.xy) * 2.0 - 1.0;
    vec3 cameraDir = vec3(0.0, 0.0, 0.8);
    vec3 cameraPlaneU = vec3(1.0, 0.0, 0.0);
    vec3 cameraPlaneV = vec3(0.0, 1.0, 0.0) * resolution.y / resolution.x;
    vec3 rayDir = cameraDir + screenPos.x * cameraPlaneU + screenPos.y * cameraPlaneV;
    vec3 rayPos = vec3(0.0, 2.0 * sin(time * 2.7), -12.0);

    rayPos.xz = rotate2d(rayPos.xz, time);
    rayDir.xz = rotate2d(rayDir.xz, time);

    vec3 mapPos = floor(rayPos);

    vec3 deltaDist = abs(vec3(length(rayDir)) / rayDir);

    ivec3 rayStep = ivec3(sign(rayDir));

    vec3 sideDist = (sign(rayDir) * (mapPos - rayPos) + (sign(rayDir) * 0.5) + 0.5) * deltaDist;

    bvec3 mask;
    vec3 maskv;

    for (int i = 0; i < MAX_RAY_STEPS; i++) {
        if (getVoxel(mapPos))
            break;

        // All components of mask are false except for the corresponding largest component
        // of sideDist, which is the axis along which the ray should be incremented.

        mask = lessThanEqual(sideDist.xyz, min(sideDist.yzx, sideDist.zxy));

        maskv = vec3(mask);

        sideDist += maskv * deltaDist;
        mapPos += maskv * rayStep;
    }

    vec3 color;
    if (mask.x) {
        color = vec3(0.5);
    }
    if (mask.y) {
        color = vec3(1.0);
    }
    if (mask.z) {
        color = vec3(0.75);
    }
    out_color.rgb = color;
}
