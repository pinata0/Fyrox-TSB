(
    name: "Shion violet explosion",
    resources: [
        (name: "fyrox_instanceData", kind: PropertyGroup([]), binding: 0),
        (name: "fyrox_cameraData", kind: PropertyGroup([]), binding: 1),
        (name: "properties", kind: PropertyGroup([
            (name: "age", kind: Float(value: 0.0)),
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
                func: BlendFunc(sfactor: SrcAlpha, dfactor: OneMinusSrcAlpha, alpha_sfactor: Zero, alpha_dfactor: One),
                equation: BlendEquation(rgb: Add, alpha: Add),
            )),
            stencil_op: StencilOp(fail: Keep, zfail: Keep, zpass: Keep, write_mask: 0xFFFF_FFFF),
            scissor_box: None,
        ),
        vertex_shader: r#"
            layout(location=0) in vec3 vertexPosition;
            out vec3 localPosition;
            out vec3 worldPosition;
            out vec3 worldNormal;
            void main() {
                localPosition=vertexPosition;
                float turbulence=sin(vertexPosition.x*13.0+properties.age*22.0)
                    *sin(vertexPosition.y*11.0-properties.age*17.0)*sin(vertexPosition.z*12.0);
                vec3 p=vertexPosition*(1.0+0.09*turbulence);
                worldPosition=(fyrox_instanceData.worldMatrix*vec4(p,1.0)).xyz;
                worldNormal=normalize(mat3(fyrox_instanceData.worldMatrix)*vertexPosition);
                gl_Position=fyrox_instanceData.worldViewProjection*vec4(p,1.0);
            }
        "#,
        fragment_shader: r#"
            in vec3 localPosition;
            in vec3 worldPosition;
            in vec3 worldNormal;
            out vec4 FragColor;
            void main() {
                vec3 p=localPosition;
                float clouds=0.5+0.5*sin(p.x*11.0+p.y*7.0+properties.age*19.0)
                    *sin(p.y*13.0-p.z*9.0-properties.age*12.0)*sin(p.z*15.0+p.x*5.0);
                float rim=pow(1.0-abs(dot(normalize(worldNormal),normalize(fyrox_cameraData.position-worldPosition))),2.0);
                float crack=pow(clouds,6.0);
                vec3 violet=mix(vec3(0.12,0.002,0.30),vec3(0.55,0.025,0.95),rim);
                vec3 color=mix(violet,vec3(0.90,0.34,1.0),crack);
                float alpha=clamp((0.38+0.65*rim+0.8*crack)*properties.opacity,0.0,0.95);
                FragColor=vec4(color*(1.0+2.2*crack),alpha*(gl_FrontFacing?1.0:0.20));
            }
        "#,
    )],
)
