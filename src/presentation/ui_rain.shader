(
 name: "Ui clover rain",
 resources: [
  (name:"fyrox_instanceData",kind:PropertyGroup([]),binding:0),
  (name:"properties",kind:PropertyGroup([
   (name:"age",kind:Float(value:0.0)),
   (name:"ring",kind:Float(value:0.0)),
  ]),binding:1),
 ],
 disabled_passes:["GBuffer","DirectionalShadow","PointShadow","SpotShadow"],
 passes:[(
  name:"Forward",
  draw_parameters:DrawParameters(
   cull_face:None,color_write:ColorMask(red:true,green:true,blue:true,alpha:true),
   depth_write:false,stencil_test:None,depth_test:Some(LessOrEqual),
   blend:Some(BlendParameters(func:BlendFunc(sfactor:SrcAlpha,dfactor:OneMinusSrcAlpha,alpha_sfactor:Zero,alpha_dfactor:One),equation:BlendEquation(rgb:Add,alpha:Add))),
   stencil_op:StencilOp(fail:Keep,zfail:Keep,zpass:Keep,write_mask:0xFFFF_FFFF),scissor_box:None,
  ),
  vertex_shader:r#"
   layout(location=0) in vec3 vertexPosition;
   layout(location=1) in vec2 vertexTexCoord;
   out float tail;
   void main(){
    vec3 p=vertexPosition;tail=vertexTexCoord.y;
    if(properties.ring<0.5){p.y+=0.05+fract(vertexTexCoord.x*0.618034-properties.age*2.4)*4.5;}
    gl_Position=fyrox_instanceData.worldViewProjection*vec4(p,1.0);
   }
  "#,
  fragment_shader:r#"
   in float tail;out vec4 FragColor;
   void main(){
    vec3 rain=mix(vec3(0.30,0.78,0.94),vec3(0.74,1.0,0.90),tail);
    FragColor=properties.ring>0.5?vec4(0.42,0.86,0.62,0.60):vec4(rain,0.68*(1.0-tail*0.65));
   }
  "#,
 )],
)
