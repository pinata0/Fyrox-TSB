(
    name: "SelectionLiveMapBackdrop",
    resources: [
        (name: "image", kind: Texture(kind: Sampler2D, fallback: White), binding: 0),
        (name: "depthTexture", kind: Texture(kind: Sampler2D, fallback: White), binding: 1),
        (name: "properties", kind: PropertyGroup([
            (name: "worldViewProjection", kind: Matrix4()),
        ]), binding: 0),
    ],
    passes: [(
        name: "Primary",
        draw_parameters: DrawParameters(
            cull_face: None,
            color_write: ColorMask(red: true, green: true, blue: true, alpha: false),
            depth_write: false, stencil_test: None, depth_test: None,
            blend: None,
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
            void main() {
                // Preserve the hero and replace only the empty preview background.
                if (texture(depthTexture, texCoord).r < 0.999999) discard;
                vec3 background = texture(image, texCoord).rgb;
                // A subtle cool shade keeps white labels readable over bright scenery.
                outColor = vec4(background * vec3(0.62, 0.68, 0.76), 1.0);
            }
        "#,
    )],
)
