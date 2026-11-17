(
    name: "Elf portal marker",
    resources: [
        (name:"fyrox_instanceData",kind:PropertyGroup([]),binding:0),
        (name:"properties",kind:PropertyGroup([
            (name:"color",kind:Vector3(value:(1.0,0.8,0.2))),
            (name:"time",kind:Float(value:0.0)),
        ]),binding:2),
    ],
    disabled_passes:["GBuffer","DirectionalShadow","PointShadow","SpotShadow"],
    passes:[(
        name:"Forward",
        draw_parameters:DrawParameters(
            cull_face:None,
            color_write:ColorMask(red:true,green:true,blue:true,alpha:true),
            depth_write:true,stencil_test:None,depth_test:Some(LessOrEqual),blend:None,
            stencil_op:StencilOp(fail:Keep,zfail:Keep,zpass:Keep,write_mask:0xFFFF_FFFF),scissor_box:None,
        ),
        vertex_shader:r#"
            layout(location=0) in vec3 vertexPosition;
            layout(location=1) in vec2 vertexTexCoord;
            out vec2 uv;
            void main() {
                uv=vertexTexCoord;
                gl_Position=fyrox_instanceData.worldViewProjection*vec4(vertexPosition,1.0);
            }
        "#,
        fragment_shader:r#"
            in vec2 uv;
            out vec4 FragColor;
            void main() {
                float r=length(uv);
                if(r>1.0) discard;
                float edge=smoothstep(0.86,0.95,r);
                float shimmer=0.7+0.3*sin(atan(uv.y,uv.x)*18.0-properties.time*5.0);
                vec3 color=properties.color*(1.0+shimmer);
                vec3 interior=properties.color*(0.05+0.05*sin(r*24.0-properties.time*3.0));
                FragColor=vec4(mix(interior,color,edge),1.0);
            }
        "#,
    )],
)
