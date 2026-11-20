(
    name: "SelectionCard",
    resources: [
        (name: "fyrox_widgetTexture", kind: Texture(kind: Sampler2D, fallback: White), binding: 0),
        (name: "fyrox_widgetData", kind: PropertyGroup([]), binding: 0),
        (name: "properties", kind: PropertyGroup([
            (name: "grayscale", kind: Float(value: 0.0)),
        ]), binding: 1),
    ],
    disabled_passes: ["GBuffer", "DirectionalShadow", "PointShadow", "SpotShadow"],
    passes: [
        (
            name: "Forward",
            // The UI renderer supplies blending, clipping and draw parameters.
            vertex_shader: r#"
                layout(location = 0) in vec2 vertexPosition;
                layout(location = 1) in vec2 vertexTexCoord;
                layout(location = 2) in vec4 vertexColor;
                out vec2 texCoord;
                out vec4 color;
                void main() {
                    texCoord = vertexTexCoord;
                    color = vertexColor;
                    vec3 p = fyrox_widgetData.worldMatrix * vec3(vertexPosition, 1.0);
                    gl_Position = fyrox_widgetData.projectionMatrix * vec4(p, 1.0);
                }
            "#,
            fragment_shader: r#"
                in vec2 texCoord;
                in vec4 color;
                out vec4 fragColor;
                void main() {
                    // Portraits and trait markers use solid white image brushes.
                    fragColor = texture(fyrox_widgetTexture, texCoord)
                        * fyrox_widgetData.solidColor * color;
                    float gray = dot(fragColor.rgb, vec3(0.299, 0.587, 0.114));
                    fragColor.rgb = mix(fragColor.rgb, vec3(gray), properties.grayscale);
                    fragColor.a *= fyrox_widgetData.opacity;
                }
            "#,
        ),
    ],
)
