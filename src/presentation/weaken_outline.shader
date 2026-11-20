(
 name:"Weaken violet silhouette",
 resources:[
  (name:"fyrox_instanceData",kind:PropertyGroup([]),binding:0),
  (name:"fyrox_boneMatrices",kind:PropertyGroup([]),binding:1),
  (name:"properties",kind:PropertyGroup([
   (name:"width",kind:Float(value:0.004)),
   (name:"opacity",kind:Float(value:1.0)),
  ]),binding:2),
 ],
 disabled_passes:["GBuffer","DirectionalShadow","PointShadow","SpotShadow"],
 passes:[(
  name:"Forward",
  draw_parameters:DrawParameters(
   cull_face:Some(Front),color_write:ColorMask(red:true,green:true,blue:true,alpha:true),
   depth_write:false,stencil_test:None,depth_test:Some(LessOrEqual),
   blend:Some(BlendParameters(func:BlendFunc(sfactor:SrcAlpha,dfactor:One,alpha_sfactor:Zero,alpha_dfactor:One),equation:BlendEquation(rgb:Add,alpha:Add))),
   stencil_op:StencilOp(fail:Keep,zfail:Keep,zpass:Keep,write_mask:0xFFFF_FFFF),scissor_box:None,
  ),
  vertex_shader:r#"
   layout(location=0) in vec3 vertexPosition;
   layout(location=2) in vec3 vertexNormal;
   layout(location=4) in vec4 boneWeights;
   layout(location=5) in vec4 boneIndices;
   void main(){
    vec4 p=vec4(vertexPosition,1.0);vec3 n=vertexNormal;
    if(fyrox_instanceData.useSkeletalAnimation){
     mat4 skin=fyrox_boneMatrices.matrices[int(boneIndices.x)]*boneWeights.x
      +fyrox_boneMatrices.matrices[int(boneIndices.y)]*boneWeights.y
      +fyrox_boneMatrices.matrices[int(boneIndices.z)]*boneWeights.z
      +fyrox_boneMatrices.matrices[int(boneIndices.w)]*boneWeights.w;
     p=skin*p;n=mat3(skin)*n;
    }
    gl_Position=fyrox_instanceData.worldViewProjection*p;
    vec2 normal=(fyrox_instanceData.worldViewProjection*vec4(n,0.0)).xy;
    gl_Position.xy+=normal/max(length(normal),0.00001)*properties.width*gl_Position.w;
   }
  "#,
  fragment_shader:r#"
   out vec4 FragColor;
   void main(){FragColor=vec4(0.85,0.12,1.7,properties.opacity);}
  "#,
 )],
)
