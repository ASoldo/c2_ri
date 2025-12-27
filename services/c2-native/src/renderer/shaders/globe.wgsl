struct Camera {
  view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;
@group(0) @binding(1)
var earth_tex: texture_2d<f32>;
@group(0) @binding(2)
var earth_sampler: sampler;
@group(0) @binding(3)
var clouds_tex: texture_2d<f32>;
@group(0) @binding(4)
var clouds_sampler: sampler;
struct Overlay {
  weather_strength: f32,
  marine_strength: f32,
  _pad: vec2<f32>,
};
@group(0) @binding(5)
var<uniform> overlay: Overlay;

struct VertexInput {
  @location(0) position: vec3<f32>,
  @location(1) normal: vec3<f32>,
  @location(2) uv: vec2<f32>,
};

struct VertexOutput {
  @builtin(position) clip_position: vec4<f32>,
  @location(0) normal: vec3<f32>,
  @location(1) uv: vec2<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
  var out: VertexOutput;
  out.clip_position = camera.view_proj * vec4<f32>(input.position, 1.0);
  out.normal = input.normal;
  out.uv = input.uv;
  return out;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
  let light_dir = normalize(vec3<f32>(0.4, 0.6, 0.8));
  let diffuse = max(dot(normalize(input.normal), light_dir), 0.0);
  let base = textureSample(earth_tex, earth_sampler, input.uv);
  let clouds = textureSample(clouds_tex, clouds_sampler, input.uv);
  let lit = base.rgb * (0.25 + 0.75 * diffuse);
  let cloud_mix = clouds.a * 0.45 * overlay.weather_strength;
  let clouded = mix(lit, clouds.rgb, cloud_mix);
  let wave = 0.5 + 0.5 * sin(input.uv.x * 40.0) * cos(input.uv.y * 28.0);
  let marine = vec3<f32>(0.08, 0.45, 0.75) * wave * overlay.marine_strength;
  let blended = mix(clouded, marine, overlay.marine_strength * 0.35);
  return vec4<f32>(blended, 1.0);
}
