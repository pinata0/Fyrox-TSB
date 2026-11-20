(
    name: "ShieldGridSphere",
    resources: [
        (name: "fyrox_instanceData", kind: PropertyGroup([]), binding: 0),
        (name: "fyrox_cameraData", kind: PropertyGroup([]), binding: 1),
        (name: "properties", kind: PropertyGroup([(name:"mercury",kind:Float(value:0.0))]), binding: 2),
    ],
    disabled_passes: ["GBuffer", "DirectionalShadow", "PointShadow", "SpotShadow"],
    passes: [(
        name: "Forward",
        draw_parameters: DrawParameters(
            cull_face: None,
            color_write: ColorMask(red: true, green: true, blue: true, alpha: true),
            depth_write: false,
            stencil_test: None,
            depth_test: Some(LessOrEqual),
            blend: Some(BlendParameters(
                func: BlendFunc(sfactor: SrcAlpha, dfactor: OneMinusSrcAlpha,
                    alpha_sfactor: One, alpha_dfactor: OneMinusSrcAlpha),
                equation: BlendEquation(rgb: Add, alpha: Add),
            )),
            stencil_op: StencilOp(fail: Keep, zfail: Keep, zpass: Keep, write_mask: 0xFFFF_FFFF),
            scissor_box: None,
        ),
        vertex_shader: r#"
            layout(location = 0) in vec3 vertexPosition;
            out vec3 localPosition;
            out vec3 worldPosition;
            out vec3 worldNormal;
            out float cameraInside;
            void main() {
                localPosition = vertexPosition;
                worldPosition = (fyrox_instanceData.worldMatrix * vec4(vertexPosition, 1.0)).xyz;
                worldNormal = normalize(mat3(fyrox_instanceData.worldMatrix) * vertexPosition);
                vec3 center = fyrox_instanceData.worldMatrix[3].xyz;
                float radius = length(fyrox_instanceData.worldMatrix[0].xyz);
                cameraInside = distance(fyrox_cameraData.position, center) < radius ? 1.0 : 0.0;
                gl_Position = fyrox_instanceData.worldViewProjection * vec4(vertexPosition, 1.0);
            }
        "#,
        fragment_shader: r#"
            in vec3 localPosition;
            in vec3 worldPosition;
            in vec3 worldNormal;
            in float cameraInside;
            out vec4 FragColor;
            void main() {
                // Object-space cubic lattice avoids a longitude seam and crowded poles.
                vec3 cell = localPosition * 5.0;
                vec3 edge = abs(fract(cell - 0.5) - 0.5);
                vec3 width = max(fwidth(cell), vec3(0.001));
                vec3 line = 1.0 - smoothstep(vec3(0.018), vec3(0.018) + width * 1.2, edge);
                float grid = max(line.x, max(line.y, line.z));
                vec3 viewDirection = normalize(fyrox_cameraData.position - worldPosition);
                float rim = pow(1.0 - abs(dot(normalize(worldNormal), viewDirection)), 3.0);
                // Inside the sphere the local player's aim remains readable.
                float visibility = cameraInside > 0.5 ? 0.08 : (gl_FrontFacing ? 1.0 : 0.35);
                float alpha = (0.018 + 0.32 * grid + 0.22 * rim) * visibility;
                vec3 tint = mix(vec3(0.20, 0.65, 0.86), vec3(0.62, 0.93, 1.0), max(grid, rim));
                if(properties.mercury>0.5) {
                    vec3 n=normalize(worldNormal);
                    float silver=pow(max(0.0,dot(n,normalize(vec3(-0.5,0.8,-0.4)))),12.0);
                    float ribbon=pow(0.5+0.5*sin(localPosition.y*8.0+localPosition.x*4.0),14.0);
                    tint=mix(vec3(0.40,0.44,0.50),vec3(0.96,0.98,1.0),clamp(rim+silver+ribbon*0.35,0.0,1.0));
                    alpha=(0.035+rim*0.58+silver*0.26+ribbon*0.065)*visibility;
                }
                FragColor = vec4(tint, alpha);
            }
        "#,
    )],
)
