(
 name:"Bella ghost fire and healing wave",
 resources:[
  (name:"fyrox_instanceData",kind:PropertyGroup([]),binding:0),
  (name:"properties",kind:PropertyGroup([
   (name:"age",kind:Float(value:0.0)),
   (name:"mode",kind:Float(value:0.0)),
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
   out vec2 uv;out vec3 local;
   void main(){
    vec3 p=vertexPosition;uv=vertexTexCoord;local=p;
    if(properties.mode<0.5){
      float h=clamp(p.y/0.32,0.0,1.0);
      p.x+=sin(properties.age*5.0+h*8.0)*h*h*0.035;
      p.z+=cos(properties.age*4.0+h*7.0)*h*h*0.030;
      p.y*=1.0+sin(properties.age*6.0)*0.09;
    }
    if(properties.mode>2.5){
      p.x+=sin(properties.age*5.0+p.z*4.0)*uv.y*uv.y*0.16;
      p.y*=0.72+0.28*sin(properties.age*7.0+p.z*3.0+p.x*9.0);
    }
    gl_Position=fyrox_instanceData.worldViewProjection*vec4(p,1.0);
   }
  "#,
  fragment_shader:r#"
   in vec2 uv;in vec3 local;out vec4 FragColor;
   void main(){
    float t=properties.age;
    if(properties.mode>2.5){
      float width=(1.0-uv.y)*(0.85+0.15*sin(uv.y*16.0+t*8.0));
      float alpha=(1.0-smoothstep(width-0.18,width,abs(uv.x*2.0-1.0)))*(1.0-uv.y);
      FragColor=vec4(mix(vec3(0.43,0.02,0.88),vec3(0.71,0.22,1.0),1.0-uv.y),alpha*0.88);
    }else if(properties.mode>1.5){
      float edge=sin(uv.y*3.14159);
      FragColor=vec4(1.6,1.65,1.8,edge*max(0.0,1.0-t/0.9)*0.8);
    }else if(properties.mode>0.5){
      float edge=max(smoothstep(1.36,1.49,abs(local.x)),max(1.0-smoothstep(0.0,0.12,local.z),smoothstep(9.88,10.0,local.z)));
      float swirl=0.5+0.5*sin(local.z*5.0+sin(local.x*7.0-t*2.0)-t*5.0);
      FragColor=vec4(0.32+edge*0.26,0.015+edge*0.12,0.62+edge*0.35,0.24+edge*0.50+swirl*0.08);
    }else{
      float h=clamp(local.y/0.32,0.0,1.0);
      vec3 c=mix(vec3(0.18,0.85,1.0),vec3(0.62,0.08,0.95),h);
      FragColor=vec4(c,0.92*(1.0-h*0.15));
    }
   }
  "#,
 )],
)
