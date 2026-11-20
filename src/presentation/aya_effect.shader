(
 name: "Aya pearl butterfly and snow garden",
 resources: [
  (name:"fyrox_instanceData",kind:PropertyGroup([]),binding:0),
  (name:"properties",kind:PropertyGroup([
   (name:"style",kind:Float(value:0.0)),
   (name:"time",kind:Float(value:0.0)),
   (name:"opacity",kind:Float(value:1.0)),
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
    vec3 p=vertexPosition;uv=vertexTexCoord;
    if(properties.style>6.5){
     float seed=floor(vertexTexCoord.x*0.5),phase=seed/220.0;
     float center=fract(seed*0.618034)*6.0;
     p.y=mod(center-properties.time*(0.75+phase*0.65)+9.0,6.0)+(p.y-center);
     p.x+=sin(properties.time*0.8+phase*43.0)*0.38;
     p.z+=cos(properties.time*0.6+phase*31.0)*0.28;
     uv=vec2(fract(vertexTexCoord.x),vertexTexCoord.y);
    }
    local=p;gl_Position=fyrox_instanceData.worldViewProjection*vec4(p,1.0);
   }
  "#,
  fragment_shader:r#"
   in vec2 uv;in vec3 local;out vec4 FragColor;
   void main(){
    float s=properties.style;vec3 c;float a=1.0;
    if(s<0.5){c=mix(vec3(.79,.94,1.0),vec3(.59,.41,.96),uv.x);a=.76;}
    else if(s<1.5){c=vec3(.80,.43,1.6);a=.95;}
    else if(s<2.5){c=vec3(.58,1.1,1.35);a=.92;}
    else if(s<3.5){c=vec3(.26,.10,.48);}
    else if(s<4.5){c=mix(vec3(.71,.86,.98),vec3(1.15),uv.x);}
    else if(s<5.5){c=vec3(1.3,1.14,.68);}
    else if(s<6.5){
     float vein=pow(max(0.0,sin(local.y*24.0+local.x*15.0+sin(local.z*21.0)*2.0)),28.0);
     c=mix(vec3(.22,.65,.92),vec3(.78,1.12,1.3),vein);a=.20+.24*vein;
    }else{
     vec2 q=(uv-.5)*2.0;float r=length(q);
     float star=.63+.16*cos(atan(q.y,q.x)*6.0);
     a=(1.0-smoothstep(star-.12,star,r))*.85;
     if(a<.01)discard;c=vec3(1.13,1.20,1.3);
    }
    FragColor=vec4(c,a*properties.opacity);
   }
  "#,
 )],
)
