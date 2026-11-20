(
    name: "EnergyBallHalo",
    resources: [
        (name: "fyrox_instanceData", kind: PropertyGroup([]), binding: 0),
        (name: "fyrox_cameraData", kind: PropertyGroup([]), binding: 1),
    ],
    disabled_passes: ["GBuffer", "DirectionalShadow", "PointShadow", "SpotShadow"],
    passes: [(
        name: "Forward",
        draw_parameters: DrawParameters(
            cull_face: Some(Back),
            color_write: ColorMask(red: true, green: true, blue: true, alpha: true),
            depth_write: false,
            stencil_test: None,
            depth_test: Some(LessOrEqual),
            blend: Some(BlendParameters(
                func: BlendFunc(sfactor: SrcAlpha, dfactor: One,
                    alpha_sfactor: Zero, alpha_dfactor: One),
                equation: BlendEquation(rgb: Add, alpha: Add),
            )),
            stencil_op: StencilOp(fail: Keep, zfail: Keep, zpass: Keep, write_mask: 0xFFFF_FFFF),
            scissor_box: None,
        ),
        vertex_shader: r#"
            layout(location = 0) in vec3 vertexPosition;
            out vec3 worldPosition;
            out vec3 worldNormal;
            void main() {
                worldPosition = (fyrox_instanceData.worldMatrix * vec4(vertexPosition, 1.0)).xyz;
                worldNormal = normalize(mat3(fyrox_instanceData.worldMatrix) * vertexPosition);
                gl_Position = fyrox_instanceData.worldViewProjection * vec4(vertexPosition, 1.0);
            }
        "#,
        fragment_shader: r#"
            in vec3 worldPosition;
            in vec3 worldNormal;
            out vec4 FragColor;
            void main() {
                vec3 viewDirection = normalize(fyrox_cameraData.position - worldPosition);
                float facing = max(dot(normalize(worldNormal), viewDirection), 0.0);
                // Soft additive falloff stays visible with the toon renderer's bloom disabled.
                float halo = pow(facing, 3.5) * 0.62;
                FragColor = vec4(0.65, 0.28, 1.0, halo);
            }
        "#,
    )],
)
