(
 name:"Frozen first person ice",
 resources:[
  (name:"fyrox_widgetTexture",kind:Texture(kind:Sampler2D,fallback:White),binding:0),
  (name:"fyrox_widgetData",kind:PropertyGroup([]),binding:0),
 ],
 disabled_passes:["GBuffer","DirectionalShadow","PointShadow","SpotShadow"],
 passes:[(
  name:"Forward",
  vertex_shader:r#"
   layout(location=0) in vec2 vertexPosition;
   layout(location=1) in vec2 vertexTexCoord;
   out vec2 uv;
   void main(){uv=vertexTexCoord;vec3 p=fyrox_widgetData.worldMatrix*vec3(vertexPosition,1.0);gl_Position=fyrox_widgetData.projectionMatrix*vec4(p,1.0);}
  "#,
  fragment_shader:r#"
   in vec2 uv;out vec4 FragColor;
   float hash(vec2 p){return fract(sin(dot(p,vec2(127.1,311.7)))*43758.5453);}
   void main(){
    vec2 q=uv*vec2(13.0,8.0),cell=floor(q),f=fract(q);float a=10.0,b=10.0;
    for(int y=-1;y<=1;y++)for(int x=-1;x<=1;x++){
     vec2 o=vec2(x,y);vec2 seed=vec2(hash(cell+o),hash(cell+o+23.7));float d=length(o+seed-f);
     if(d<a){b=a;a=d;}else if(d<b){b=d;}
    }
    float cracks=1.0-smoothstep(.008,.055,b-a);
    float edge=smoothstep(.22,.68,length((uv-.5)*vec2(1.1,1.0)));
    float grain=hash(floor(uv*950.0));
    vec3 color=mix(vec3(.31,.66,.86),vec3(.88,.97,1.0),max(cracks*.82,edge*.68));
    float opacity=.14+edge*.55+cracks*(.10+edge*.22)+grain*.025*edge;
    FragColor=vec4(color,opacity*fyrox_widgetData.opacity);
   }
  "#,
 )],
)
