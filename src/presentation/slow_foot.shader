(
 name:"Slow blue ankle haze",
 resources:[
  (name:"fyrox_instanceData",kind:PropertyGroup([]),binding:0),
  (name:"properties",kind:PropertyGroup([
   (name:"strength",kind:Float(value:1.0)),
   (name:"time",kind:Float(value:0.0)),
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
   out vec3 local;
   void main(){local=vertexPosition;gl_Position=fyrox_instanceData.worldViewProjection*vec4(vertexPosition,1.0);}
  "#,
  fragment_shader:r#"
   in vec3 local;out vec4 FragColor;
   void main(){
    float pulse=.93+.07*sin(properties.time*3.0);
    float alpha;
    if(local.y<.06){
     float r=length(local.xz)/.68;
     float halo=1.0-smoothstep(.36,1.0,r);
     float ring=smoothstep(.58,.72,r)*(1.0-smoothstep(.76,.91,r));
     alpha=.55*halo+.65*ring;
    }else{
     alpha=.50*pow(1.0-clamp(local.y/.46,0.0,1.0),1.6);
    }
    FragColor=vec4(.08,.36,1.0,alpha*pulse*(.60+.40*properties.strength));
   }
  "#,
 )],
)
