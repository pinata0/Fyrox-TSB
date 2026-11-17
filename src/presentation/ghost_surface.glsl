// Runs inside the scenery's existing GBuffer fragment pass. No extra draw,
// displaced vertices, proxy faces or depth changes are needed for the ripples.
vec3 rippleNormal = normalize(outNormal.xyz * 2.0 - 1.0);
float verticalSurface = 1.0 - smoothstep(0.65, 0.95, abs(normalize(normal).y));
for (int i = 0; i < properties.ghostCount; ++i) {
    vec3 offset = position - properties.ghostCenters[i].xyz;
    vec3 scaled = offset / vec3(1.45, 1.3, 1.45);
    float radius = length(scaled);
    if (radius >= 1.0 || verticalSurface <= 0.0) {
        continue;
    }
    float fade = properties.ghostCenters[i].w * verticalSurface;
    float envelope = (1.0 - radius) * (1.0 - radius) * fade;
    float phase = radius * 24.0 - properties.ghostTimes[i].x * 14.0;
    vec3 radial = scaled / max(radius, 0.001);
    vec3 alongSurface = radial - rippleNormal * dot(radial, rippleNormal);
    rippleNormal = normalize(rippleNormal + alongSurface * cos(phase) * envelope * 0.65);
    float ring = pow(0.5 + 0.5 * sin(phase), 10.0) * envelope;
    outColor.rgb = mix(outColor.rgb, vec3(0.33, 0.91, 0.76), ring * 0.48);
    outAmbient.rgb += vec3(0.08, 0.42, 0.30) * ring * 0.5;
}
outNormal.xyz = rippleNormal * 0.5 + 0.5;
