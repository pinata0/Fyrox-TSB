(
    name: "ShionScopeComposite",
    resources: [
        (name: "image", kind: Texture(kind: Sampler2D, fallback: White), binding: 0),
        (
            name: "properties",
            kind: PropertyGroup([
                (name: "worldViewProjection", kind: Matrix4()),
                (name: "screenSize", kind: Vector2()),
                (name: "lensRadius", kind: Float()),
                (name: "feather", kind: Float()),
            ]),
            binding: 0,
        ),
    ],
    passes: [
        (
            name: "Primary",
            draw_parameters: DrawParameters(
                cull_face: None,
                color_write: ColorMask(red: true, green: true, blue: true, alpha: false),
                depth_write: false,
                stencil_test: None,
                depth_test: None,
                blend: Some(BlendParameters(
                    func: BlendFunc(
                        sfactor: SrcAlpha, dfactor: OneMinusSrcAlpha,
                        alpha_sfactor: Zero, alpha_dfactor: One,
                    ),
                    equation: BlendEquation(rgb: Add, alpha: Add),
                )),
                stencil_op: StencilOp(
                    fail: Keep, zfail: Keep, zpass: Keep, write_mask: 0xFFFF_FFFF,
                ),
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
                void main() {
                    // Pixel distances keep the lens circular on wide and tall windows.
                    float distanceToCenter = length((texCoord - 0.5) * properties.screenSize);
                    float outside = smoothstep(
                        properties.lensRadius - properties.feather,
                        properties.lensRadius + properties.feather,
                        distanceToCenter
                    );
                    if (outside <= 0.0) {
                        discard;
                    }
                    // A light peripheral dim keeps attention on the clear lens without hiding scenery.
                    vec3 peripheral = texture(image, texCoord).rgb * 0.9;
                    outColor = vec4(peripheral, outside);
                }
            "#,
        ),
    ],
)
