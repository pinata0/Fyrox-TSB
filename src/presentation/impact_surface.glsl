// Shade the actual scenery fragments so dents follow the visible mesh instead
// of drawing floating polygons on its simplified collision boxes.
vec3 dentNormal = normalize(outNormal.xyz * 2.0 - 1.0);
for (int i = 0; i < properties.impactCount; ++i) {
    vec3 axis = properties.impactNormals[i].xyz;
    vec3 delta = position - properties.impactCenters[i].xyz;
    float depth = dot(delta, axis);
    float facing = dot(normalize(normal), axis);
    if (abs(depth) > 0.38 || facing < 0.35) { continue; }
    vec3 tangent = normalize(cross(axis, abs(axis.y) > 0.8 ? vec3(1, 0, 0) : vec3(0, 1, 0)));
    vec3 bitangent = cross(axis, tangent);
    vec2 uv = vec2(dot(delta, tangent), dot(delta, bitangent)) / properties.impactCenters[i].w;
    float fade = properties.impactNormals[i].w * smoothstep(0.35, 0.7, facing);
    float seed = dot(properties.impactCenters[i].xyz, vec3(7.13, 3.71, 11.97));
    if (properties.impactShapes[i].x > 0.5) {
        // A narrow recessed cut with tapered tips and a chipped, lit bevel.
        vec2 stroke = properties.impactShapes[i].yz;
        vec2 across = vec2(-stroke.y, stroke.x);
        float along = dot(uv, stroke);
        if (abs(along) >= 1.0) { continue; }
        float taper = sqrt(max(0.0, 1.0 - along * along));
        float side = dot(uv, across) - 0.028 * (1.0 - along * along);
        float width = properties.impactShapes[i].w * taper * (1.0 + 0.08 * sin(along * 43.0 + seed));
        float r = abs(side) / max(width, 0.002);
        if (r >= 2.0) { continue; }
        float ends = 1.0 - smoothstep(0.82, 1.0, abs(along));
        float cut = (1.0 - smoothstep(0.25, 0.95, r)) * ends * fade;
        float bevel = smoothstep(0.65, 1.0, r) * (1.0 - smoothstep(1.15, 2.0, r)) * ends * fade;
        outColor.rgb *= 1.0 - cut * 0.82;
        outColor.rgb = mix(outColor.rgb, outColor.rgb * 0.78 + vec3(0.09), bevel * 0.75);
        vec3 acrossWorld = tangent * across.x + bitangent * across.y;
        dentNormal = normalize(dentNormal - acrossWorld * sign(side) * (cut * 0.85 - bevel * 0.4));
        continue;
    }
    float angle = atan(uv.y, uv.x);
    float edge = 0.86 + 0.09 * sin(angle * 5.0 + seed + sin(angle * 3.0)) + 0.05 * cos(angle * 9.0 - seed);
    float r = length(uv) / edge;
    if (r >= 1.2) { continue; }
    float cavity = (1.0 - smoothstep(0.26, 0.77, r)) * fade;
    float rim = smoothstep(0.55, 0.79, r) * (1.0 - smoothstep(0.88, 1.12, r)) * fade;
    float pits = 0.95 + 0.05 * sin(uv.x * 17.0 + sin(uv.y * 13.0)) * cos(uv.y * 11.0 + uv.x * 5.0);
    outColor.rgb *= 1.0 - cavity * 0.83 * pits;
    outColor.rgb = mix(outColor.rgb, outColor.rgb * 0.68 + vec3(0.10), rim * 0.65);
    vec3 radial = tangent * uv.x + bitangent * uv.y;
    dentNormal = normalize(dentNormal - radial * (cavity * 1.7 - rim * 0.65));
}
outNormal.xyz = dentNormal * 0.5 + 0.5;
