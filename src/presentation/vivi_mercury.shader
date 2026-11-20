(
 name:"Vivi liquid mercury",
 resources:[
  (name:"fyrox_instanceData",kind:PropertyGroup([]),binding:0),
  (name:"properties",kind:PropertyGroup([(name:"age",kind:Float(value:0.0)),(name:"splash",kind:Float(value:0.0))]),binding:1),
 ],
 disabled_passes:["GBuffer","DirectionalShadow","PointShadow","SpotShadow"],
 passes:[(
  name:"Forward",
  draw_parameters:DrawParameters(
   cull_face:None,color_write:ColorMask(red:true,green:true,blue:true,alpha:true),depth_write:false,stencil_test:None,depth_test:Some(LessOrEqual),
   blend:Some(BlendParameters(func:BlendFunc(sfactor:SrcAlpha,dfactor:OneMinusSrcAlpha,alpha_sfactor:One,alpha_dfactor:OneMinusSrcAlpha),equation:BlendEquation(rgb:Add,alpha:Add))),
   stencil_op:StencilOp(fail:Keep,zfail:Keep,zpass:Keep,write_mask:0xFFFF_FFFF),scissor_box:None,
  ),
  vertex_shader:r#"
   layout(location=0) in vec3 vertexPosition;
   out vec3 localPosition;
   void main(){
    vec3 p=vertexPosition;localPosition=p;
    if(properties.splash<0.5 && properties.age>=0.9)p.y+=0.012*sin(p.x*4.0+properties.age*3.0)*sin(p.z*3.5-properties.age*2.0);
    gl_Position=fyrox_instanceData.worldViewProjection*vec4(p,1.0);
   }
  "#,
  fragment_shader:r#"
   in vec3 localPosition;out vec4 FragColor;
   void main(){
    float r=length(localPosition.xz);float t=properties.age-0.9;
    if(properties.splash>0.5){FragColor=vec4(mix(vec3(0.40,0.44,0.50),vec3(0.95,0.97,1.0),localPosition.y),0.8*(1.0-smoothstep(0.3,0.65,t)));return;}
    float edge=1.0-smoothstep(3.90,4.0,r);
    float rim=smoothstep(3.82,3.88,r)*edge;
    if(t<0.0){float pulse=0.55+0.25*sin(properties.age*16.0);FragColor=vec4(vec3(0.80,0.86,0.94),rim*pulse+edge*0.055);return;}
    float ripple=sin(r*10.0-t*10.0+sin(localPosition.x*3.0+t*2.0)*0.5);
    float flow=sin(localPosition.x*2.6+localPosition.z*1.8+t*1.4)*sin(localPosition.z*3.0-t);
    float reflection=pow(0.5+0.5*sin(localPosition.x*1.5+localPosition.z*2.0+flow*1.6+t*0.8),6.0);
    vec3 silver=mix(vec3(0.27,0.30,0.36),vec3(0.92,0.95,0.99),clamp(0.40+flow*0.16+reflection*0.62+ripple*0.10,0.0,1.0));
    silver=mix(silver,vec3(0.98),rim*0.75);
    float fade=1.0-smoothstep(4.60,5.0,t);
    FragColor=vec4(silver,(0.82*edge+0.12*rim)*fade);
   }
  "#,
 )],
)
