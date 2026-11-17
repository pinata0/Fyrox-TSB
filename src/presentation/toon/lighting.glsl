// Three broad light bands; only a narrow derivative-aware transition is blended.
// Quantize illumination, never the painted albedo (eyes, blush and costume colors).
vec3 TSB_ToonLight(TPBRContext ctx, float visibility)
{
    float light = max(dot(ctx.fragmentNormal, ctx.fragmentToLight), 0.0)
        * smoothstep(0.25, 0.75, visibility);
    float feather = max(fwidth(light) * 0.75, 0.012);
    float middle = smoothstep(0.08 - feather, 0.08 + feather, light);
    float bright = smoothstep(0.62 - feather, 0.62 + feather, light);
    // Matte ink-and-paint response, including metal props: no moving white PBR glints.
    return ctx.albedo * ctx.lightColor * (0.115 * middle + 0.105 * bright);
}
