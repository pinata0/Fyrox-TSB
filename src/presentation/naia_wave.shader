(
    name: "Naia rolling ultimate wave",
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
                float height=clamp(p.y/3.8,0.0,1.0);
                p.z+=sin(p.x*3.2+p.y*2.8-t*5.5)*0.075*height;
                p.y+=sin(p.x*4.1-t*6.2)*0.055*height;
                worldPosition=(fyrox_instanceData.worldMatrix*vec4(p,1.0)).xyz;
                gl_Position=fyrox_instanceData.worldViewProjection*vec4(p,1.0);
            }
        "#,
        fragment_shader: r#"
            in vec2 uv;
            in vec3 worldPosition;
            out vec4 FragColor;
            void main() {
                float flow=uv.y*46.0+uv.x*13.0-properties.time*7.0;
                float streak=pow(0.5+0.5*sin(flow+sin(uv.y*37.0+flow*0.17)*2.0),8.0);
                vec3 normal=normalize(cross(dFdx(worldPosition),dFdy(worldPosition)));
                vec3 view=normalize(fyrox_cameraData.position-worldPosition);
                float rim=pow(1.0-abs(dot(normal,view)),2.0);
                float foam=properties.foam;
                vec3 liquid=mix(vec3(0.025,0.40,0.65),vec3(0.26,0.86,0.96),0.3+rim*0.7);
                liquid+=vec3(0.20,0.34,0.36)*streak;
                vec3 color=mix(liquid,vec3(0.76,0.97,1.0),foam);
                float breakup=smoothstep(0.15,0.80,0.5+0.5*sin(flow*0.73+uv.y*18.0)+0.22*sin(flow*2.3));
                float alpha=mix(0.62+rim*0.22+streak*0.10,(0.72+0.28*streak)*(0.65+0.35*breakup),foam);
                FragColor=vec4(color,alpha);
            }
        "#,
    )],
)
