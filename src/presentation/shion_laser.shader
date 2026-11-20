(
    name: "Shion scoped violet laser",
    resources: [
        (name: "fyrox_instanceData", kind: PropertyGroup([]), binding: 0),
        (name: "properties", kind: PropertyGroup([
            (name: "opacity", kind: Float(value: 1.0)),
        ]), binding: 1),
    ],
    disabled_passes: ["GBuffer", "DirectionalShadow", "PointShadow", "SpotShadow"],
    passes: [(
        name: "Forward",
        draw_parameters: DrawParameters(
            cull_face: None,
            color_write: ColorMask(red: true, green: true, blue: true, alpha: true),
            depth_write: false, stencil_test: None, depth_test: Some(LessOrEqual),
            blend: Some(BlendParameters(
                func: BlendFunc(sfactor: SrcAlpha, dfactor: One, alpha_sfactor: Zero, alpha_dfactor: One),
                equation: BlendEquation(rgb: Add, alpha: Add),
            )),
            stencil_op: StencilOp(fail: Keep, zfail: Keep, zpass: Keep, write_mask: 0xFFFF_FFFF),
            scissor_box: None,
        ),
        vertex_shader: r#"
            layout(location=0) in vec3 vertexPosition;
            layout(location=1) in vec2 vertexTexCoord;
            out vec2 uv;
            void main() {
                uv = vertexTexCoord;
                gl_Position = fyrox_instanceData.worldViewProjection * vec4(vertexPosition, 1.0);
            }
        "#,
        fragment_shader: r#"
            in vec2 uv;
            out vec4 FragColor;
            void main() {
                float d = abs(uv.x * 2.0 - 1.0);
                float core = exp(-d * d * 100.0);
                float glow = exp(-d * d * 5.0) * (1.0 - smoothstep(0.65, 1.0, d));
                vec3 color = mix(vec3(0.48, 0.035, 1.0), vec3(1.5, 1.15, 1.8), core);
                FragColor = vec4(color, clamp(core + glow * 0.65, 0.0, 1.0) * properties.opacity);
            }
        "#,
    )],
)
