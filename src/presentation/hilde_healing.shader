(
    name: "Hilde rising healing crosses",
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
            layout(location=1) in vec2 vertexTexCoord;
            out vec2 uv;
            out float fade;
            void main() {
                float seed=floor(vertexTexCoord.x/4.0);
                uv=vec2(vertexTexCoord.x-seed*4.0,vertexTexCoord.y);
                float phase=fract(properties.age/1.15+seed*0.618034);
                float size=0.30+0.14*fract(seed*0.381966);
                vec3 center=vec3(vertexPosition.x-(uv.x-0.5)*0.7,0.0,vertexPosition.z);
                vec3 worldCenter=(fyrox_instanceData.worldMatrix*vec4(center,1.0)).xyz;
                vec3 toCamera=fyrox_cameraData.position-worldCenter;
                vec3 right=normalize(vec3(toCamera.z,0.0,-toCamera.x)+vec3(0.00001,0.0,0.0));
                vec3 p=center+right*(uv.x-0.5)*size;
                p.y=0.30+phase*1.65+(uv.y-0.5)*size;
                fade=smoothstep(0.0,0.16,phase)*(1.0-smoothstep(0.65,1.0,phase));
                // Fade close crosses before they can fill the first-person view.
                vec3 particleCenter=worldCenter+vec3(0.0,0.30+phase*1.65,0.0);
                fade*=smoothstep(0.85,1.65,length(fyrox_cameraData.position-particleCenter));
                gl_Position=fyrox_instanceData.worldViewProjection*vec4(p,1.0);
            }
        "#,
        fragment_shader: r#"
            in vec2 uv;
            in float fade;
            out vec4 FragColor;
            void main() {
                vec2 q=abs((uv-0.5)*2.0);
                // Union of two bars gives a clean medical plus, with a soft edge.
                float shape=min(max(q.x-0.25,q.y-0.82),max(q.x-0.82,q.y-0.25));
                float edge=max(fwidth(shape),0.012);
                float cross=1.0-smoothstep(-edge,edge,shape);
                float glow=exp(-max(shape,0.0)*18.0)*0.13;
                float alpha=(cross*0.86+glow)*fade*properties.opacity;
                if(alpha<0.005)discard;
                vec3 color=mix(vec3(0.025,0.72,0.08),vec3(0.24,1.0,0.32),cross);
                FragColor=vec4(color,alpha);
            }
        "#,
    )],
)
