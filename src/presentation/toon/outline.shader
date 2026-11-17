(
    name: "TSBToonInk",
    resources: [
        (name: "depthTexture", kind: Texture(kind: Sampler2D, fallback: White), binding: 0),
        (name: "normalTexture", kind: Texture(kind: Sampler2D, fallback: Normal), binding: 1),
        (name: "properties", kind: PropertyGroup([
            (name: "worldViewProjection", kind: Matrix4()),
            (name: "inverseProjection", kind: Matrix4()),
            (name: "screenSize", kind: Vector2()),
            (name: "lineWidth", kind: Float()),
        ]), binding: 0),
    ],
    passes: [(
        name: "Primary",
        draw_parameters: DrawParameters(
            cull_face: None,
            color_write: ColorMask(red: true, green: true, blue: true, alpha: false),
            depth_write: false, stencil_test: None, depth_test: None,
            blend: Some(BlendParameters(
                func: BlendFunc(sfactor: SrcAlpha, dfactor: OneMinusSrcAlpha,
                    alpha_sfactor: Zero, alpha_dfactor: One),
                equation: BlendEquation(rgb: Add, alpha: Add),
            )),
            stencil_op: StencilOp(fail: Keep, zfail: Keep, zpass: Keep, write_mask: 0xFFFF_FFFF),
            scissor_box: None,
        ),
        vertex_shader: r#"
            layout(location = 0) in vec3 vertexPosition;
            layout(location = 1) in vec2 vertexTexCoord;
            out vec2 texCoord;
            void main() {
                texCoord = vertexTexCoord;
                gl_Position = properties.worldViewProjection * vec4(vertexPosition, 1.0);
            }
        "#,
        fragment_shader: r#"
            in vec2 texCoord;
            out vec4 outColor;

            float viewDepth(vec2 uv, float depth) {
                vec4 p = properties.inverseProjection * vec4(uv * 2.0 - 1.0, depth * 2.0 - 1.0, 1.0);
                return abs(p.z / p.w);
            }
            void main() {
                float depth = texture(depthTexture, texCoord).r;
                // Draw inside geometry only, so sky and transparent empty pixels stay clean.
                if (depth >= 0.999999) discard;
                float z = viewDepth(texCoord, depth);
                vec3 n = normalize(texture(normalTexture, texCoord).xyz * 2.0 - 1.0);
                vec2 pixel = properties.lineWidth / properties.screenSize;
                float silhouette = 0.0;
                float crease = 0.0;
                // Opposite-depth differences cancel a flat surface's perspective slope.
                // This avoids stripes across floors and a wireframe over rounded cheeks.
                for (int axis = 0; axis < 2; ++axis) {
                    vec2 offset = axis == 0 ? vec2(pixel.x, 0.0) : vec2(0.0, pixel.y);
                    float da = texture(depthTexture, texCoord + offset).r;
                    float db = texture(depthTexture, texCoord - offset).r;
                    float za = viewDepth(texCoord + offset, da);
                    float zb = viewDepth(texCoord - offset, db);
                    float jump = max(abs(za - z), abs(zb - z));
                    float curvature = abs(za + zb - 2.0 * z);
                    float threshold = max(0.012, z * 0.008);
                    silhouette = max(silhouette,
                        smoothstep(threshold, threshold * 2.0, min(jump, curvature)));
                    vec3 na = normalize(texture(normalTexture, texCoord + offset).xyz * 2.0 - 1.0);
                    vec3 nb = normalize(texture(normalTexture, texCoord - offset).xyz * 2.0 - 1.0);
                    float bend = max(1.0 - dot(n, na), 1.0 - dot(n, nb));
                    crease = max(crease, smoothstep(0.48, 0.78, bend)
                        * (1.0 - smoothstep(threshold, threshold * 3.0, jump)));
                }
                // Fine inner creases, stronger outer contour. Distant detail fades smoothly.
                float distanceFade = 1.0 - smoothstep(8.0, 48.0, z);
                float ink = max(silhouette * 0.88, crease * 0.34) * distanceFade;
                if (ink < 0.01) discard;
                outColor = vec4(S_SRGBToLinear(vec4(0.20, 0.105, 0.145, 1.0)).rgb, ink);
            }
        "#,
    )],
)
