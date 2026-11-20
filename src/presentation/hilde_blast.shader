(
    name: "Hilde green explosion",
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
                float turbulence=sin(vertexPosition.x*11.0+properties.age*22.0)
                    *sin(vertexPosition.y*9.0-properties.age*17.0)*sin(vertexPosition.z*12.0);
                vec3 p=vertexPosition*(1.0+0.08*turbulence);
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
                float clouds=0.5+0.5*sin(p.x*9.0+p.y*7.0+properties.age*19.0)
                    *sin(p.y*11.0-p.z*9.0-properties.age*12.0)*sin(p.z*13.0+p.x*5.0);
                float rim=pow(1.0-abs(dot(normalize(worldNormal),normalize(fyrox_cameraData.position-worldPosition))),2.0);
                float spark=pow(clouds,5.0);
                vec3 green=mix(vec3(0.025,0.48,0.055),vec3(0.16,1.0,0.25),rim);
                vec3 color=mix(green,vec3(0.55,1.0,0.40),spark);
                float alpha=clamp((0.30+0.60*rim+0.65*spark)*properties.opacity,0.0,0.9);
                FragColor=vec4(color*(1.0+0.8*spark),alpha*(gl_FrontFacing?1.0:0.18));
            }
        "#,
    )],
)
