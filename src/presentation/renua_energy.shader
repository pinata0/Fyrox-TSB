(
    name: "Renua temporal energy",
    resources: [
        (name: "fyrox_instanceData", kind: PropertyGroup([]), binding: 0),
        (name: "fyrox_cameraData", kind: PropertyGroup([]), binding: 1),
        (name: "properties", kind: PropertyGroup([
            (name: "blue", kind: Float(value: 0.0)),
            (name: "opacity", kind: Float(value: 1.0)),
        ]), binding: 2),
    ],
    disabled_passes: ["GBuffer", "DirectionalShadow", "PointShadow", "SpotShadow"],
    passes: [(
        name: "Forward",
        draw_parameters: DrawParameters(
            cull_face: None,
            color_write: ColorMask(red: true, green: true, blue: true, alpha: true),
            depth_write: false, stencil_test: None, depth_test: Some(LessOrEqual),
            blend: Some(BlendParameters(
                func: BlendFunc(sfactor: SrcAlpha, dfactor: OneMinusSrcAlpha,
                    alpha_sfactor: Zero, alpha_dfactor: One),
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
                uv=vertexTexCoord;
                gl_Position=fyrox_instanceData.worldViewProjection*vec4(vertexPosition,1.0);
            }
        "#,
        fragment_shader: r#"
            in vec2 uv;
            out vec4 FragColor;
            void main() {
                float d=abs(uv.x*2.0-1.0);
                float core=exp(-d*d*170.0);
                float halo=exp(-d*d*7.0)*(1.0-smoothstep(0.7,1.0,d));
                vec3 gold=mix(vec3(1.0,0.63,0.005),vec3(1.0,0.87,0.18),core*0.45);
                vec3 blue=vec3(0.04,0.55,1.0)*(1.0+core*1.8);
                FragColor=vec4(mix(gold,blue,properties.blue),
                    clamp(core+halo*0.85,0.0,1.0)*properties.opacity);
            }
        "#,
    )],
)
