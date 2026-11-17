// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

// Adapted from Fyrox 1.0.1. Resource bindings and shadow sampling are retained.
(
    name: "TSBToonAmbientLight",
    resources: [
        (
            name: "diffuseTexture",
            kind: Texture(kind: Sampler2D, fallback: White),
            binding: 0
        ),
        (
            name: "aoSampler",
            kind: Texture(kind: Sampler2D, fallback: White),
            binding: 1
        ),
        (
            name: "bakedLightingTexture",
            kind: Texture(kind: Sampler2D, fallback: White),
            binding: 2
        ),
        (
            name: "depthTexture",
            kind: Texture(kind: Sampler2D, fallback: White),
            binding: 3
        ),
        (
            name: "normalTexture",
            kind: Texture(kind: Sampler2D, fallback: White),
            binding: 4
        ),
        (
            name: "materialTexture",
            kind: Texture(kind: Sampler2D, fallback: White),
            binding: 5
        ),
        (
            name: "prefilteredSpecularMap",
            kind: Texture(kind: SamplerCube, fallback: White),
            binding: 6
        ),
        (
            name: "irradianceMap",
            kind: Texture(kind: SamplerCube, fallback: White),
            binding: 7
        ),
        (
            name: "brdfLUT",
            kind: Texture(kind: Sampler2D, fallback: White),
            binding: 8
        ),
        (
            name: "properties",
            kind: PropertyGroup([
                (name: "worldViewProjection", kind: Matrix4()),
                (name: "ambientColor", kind: Vector4()),
                (name: "cameraPosition", kind: Vector3()),
                (name: "invViewProj", kind: Matrix4()),
                (name: "skyboxLighting", kind: Bool()),
                (name: "environmentLightingBrightness", kind: Float()),
            ]),
            binding: 0
        ),
    ],
    passes: [
        (
            name: "Primary",

            draw_parameters: DrawParameters(
                cull_face: None,
                color_write: ColorMask(
                    red: true,
                    green: true,
                    blue: true,
                    alpha: true,
                ),
                depth_write: false,
                stencil_test: None,
                depth_test: None,
                blend: Some(BlendParameters(
                    func: BlendFunc(
                        sfactor: SrcAlpha,
                        dfactor: OneMinusSrcAlpha,
                        alpha_sfactor: SrcAlpha,
                        alpha_dfactor: OneMinusSrcAlpha,
                    ),
                    equation: BlendEquation(
                        rgb: Add,
                        alpha: Add
                    )
                )),
                stencil_op: StencilOp(
                    fail: Keep,
                    zfail: Keep,
                    zpass: Keep,
                    write_mask: 0xFFFF_FFFF,
                ),
                scissor_box: None
            ),

            vertex_shader:
                r#"
                    layout (location = 0) in vec3 vertexPosition;
                    layout (location = 1) in vec2 vertexTexCoord;

                    out vec2 texCoord;

                    void main()
                    {
                        texCoord = vertexTexCoord;
                        gl_Position = properties.worldViewProjection * vec4(vertexPosition, 1.0);
                    }
                "#,

            fragment_shader:
                r#"
                    out vec4 FragColor;
                    in vec2 texCoord;

                    void main()
                    {
                        float depth = texture(depthTexture, texCoord).r;
                        vec3 fragmentPosition = S_UnProject(vec3(texCoord, depth), properties.invViewProj);

                        vec4 albedo = S_SRGBToLinear(texture(diffuseTexture, texCoord));

                        // Flat, chromatic fill keeps dark sides and painted faces readable.
                        // Material/SSAO only supplies a gentle contact accent, never black cavities.
                        float ao = texture(aoSampler, texCoord).r * texture(materialTexture, texCoord).b;
                        float contact = mix(0.90, 1.0, smoothstep(0.25, 0.85, ao));
                        vec4 bakedLighting = texture(bakedLightingTexture, texCoord);
                        vec3 fill = properties.ambientColor.rgb * vec3(1.04, 0.94, 1.0);
                        fill *= properties.environmentLightingBrightness;
                        FragColor.rgb = albedo.rgb * (fill * contact + bakedLighting.rgb);
                        FragColor.a = bakedLighting.a;
                    }
                "#,
        )
    ]
)