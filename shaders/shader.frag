// https://www.shadertoy.com/view/XsVSzW

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
    vec2 frag_coord = gl_FragCoord.xy;
    vec2 resolution = vec2(constants.res_x, constants.res_y);

    vec2 uv = (frag_coord.xy / resolution.xx-0.5)*8.0;
    vec2 uv0=uv;
    float i0=1.0;
    float i1=1.0;
    float i2=1.0;
    float i4=0.0;
    for(int s=0;s<7;s++)
    {
        vec2 r;
        r=vec2(cos(uv.y*i0-i4+time/i1),sin(uv.x*i0-i4+time/i1))/i2;
        r+=vec2(-r.y,r.x)*0.3;
        uv.xy+=r;

        i0*=1.93;
        i1*=1.15;
        i2*=1.7;
        i4+=0.05+0.1*time*i1;
    }
    float r=sin(uv.x-time)*0.5+0.5;
    float b=sin(uv.y+time)*0.5+0.5;
    float g=sin((uv.x+uv.y+sin(time*0.5))*0.5)*0.5+0.5;

    out_color = vec4(r,g,b,1.0);
}
