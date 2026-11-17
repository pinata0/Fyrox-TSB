(
    name: "Naia flowing healing water",
    resources: [
        (name: "fyrox_instanceData", kind: PropertyGroup([]), binding: 0),
        (name: "fyrox_cameraData", kind: PropertyGroup([]), binding: 1),
        (name: "properties", kind: PropertyGroup([
            (name: "time", kind: Float(value: 0.0)),
            (name: "foam", kind: Float(value: 0.0)),
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
                    alpha_sfactor: One, alpha_dfactor: OneMinusSrcAlpha),
                equation: BlendEquation(rgb: Add, alpha: Add),
            )),
            stencil_op: StencilOp(fail: Keep, zfail: Keep, zpass: Keep, write_mask: 0xFFFF_FFFF),
            scissor_box: None,
        ),
        vertex_shader: r#"
            layout(location=0) in vec3 vertexPosition;
            layout(location=1) in vec2 vertexTexCoord;
            out vec2 uv;
            out vec3 worldPosition;
            void main() {
                uv=vertexTexCoord;
                vec3 p=vertexPosition;
                float t=properties.time;
                float envelope=sin(clamp(p.z,0.0,1.0)*3.14159);
                p.xy*=1.0+0.12*sin(p.z*56.0-t*16.0);
                p.xy+=envelope*vec2(sin(p.z*27.0-t*9.0),cos(p.z*22.0-t*11.0))*0.035;
                worldPosition=(fyrox_instanceData.worldMatrix*vec4(p,1.0)).xyz;
                gl_Position=fyrox_instanceData.worldViewProjection*vec4(p,1.0);
            }
        "#,
        fragment_shader: r#"
            in vec2 uv;
            in vec3 worldPosition;
            out vec4 FragColor;
            void main() {
                float flow=uv.x*80.0-properties.time*24.0;
                float streak=pow(0.5+0.5*sin(flow+sin(uv.y*37.0+flow*0.17)*2.0),8.0);
                vec3 normal=normalize(cross(dFdx(worldPosition),dFdy(worldPosition)));
                vec3 view=normalize(fyrox_cameraData.position-worldPosition);
                float rim=pow(1.0-abs(dot(normal,view)),2.0);
                float foam=properties.foam;
                vec3 liquid=mix(vec3(0.025,0.40,0.65),vec3(0.26,0.86,0.96),0.3+rim*0.7);
                liquid+=vec3(0.20,0.34,0.36)*streak;
                vec3 color=mix(liquid,vec3(0.76,0.97,1.0),foam);
                float breakup=smoothstep(0.15,0.80,0.5+0.5*sin(flow*0.73+uv.y*18.0)+0.22*sin(flow*2.3));
                float alpha=mix(0.40+rim*0.35+streak*0.15,(0.40+0.48*streak)*breakup,foam);
                FragColor=vec4(color,alpha);
            }
        "#,
    )],
)
