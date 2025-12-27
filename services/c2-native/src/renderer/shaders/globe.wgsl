struct Camera {
  view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;
@group(0) @binding(1)
var base_tex: texture_2d<f32>;
@group(0) @binding(2)
var base_sampler: sampler;
@group(0) @binding(3)
var map_tex: texture_2d<f32>;
@group(0) @binding(4)
var map_sampler: sampler;
@group(0) @binding(5)
var weather_tex: texture_2d<f32>;
@group(0) @binding(6)
var weather_sampler: sampler;
@group(0) @binding(7)
var sea_tex: texture_2d<f32>;
@group(0) @binding(8)
var sea_sampler: sampler;
struct Overlay {
  base_opacity: f32,
  map_opacity: f32,
  weather_opacity: f32,
  sea_opacity: f32,
};
@group(0) @binding(9)
var<uniform> overlay: Overlay;

struct VertexInput {
  @location(0) position: vec3<f32>,
  @location(1) normal: vec3<f32>,
  @location(2) uv_equirect: vec2<f32>,
  @location(3) uv_merc: vec2<f32>,
};

struct VertexOutput {
  @builtin(position) clip_position: vec4<f32>,
  @location(0) normal: vec3<f32>,
  @location(1) uv_equirect: vec2<f32>,
  @location(2) uv_merc: vec2<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
  var out: VertexOutput;
  out.clip_position = camera.view_proj * vec4<f32>(input.position, 1.0);
  out.normal = input.normal;
  out.uv_equirect = input.uv_equirect;
  out.uv_merc = input.uv_merc;
  return out;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
  let base = textureSample(base_tex, base_sampler, input.uv_equirect);
  let base_rgb = base.rgb * overlay.base_opacity;
  let pole_mask = 1.0 - step(0.9962721, abs(input.normal.y));
  var with_map = base_rgb;
  let map_opacity = overlay.map_opacity * pole_mask;
  if (map_opacity > 0.001) {
    let map = textureSample(map_tex, map_sampler, input.uv_merc);
    let map_mix = map.a * map_opacity;
    with_map = mix(with_map, map.rgb, map_mix);
  }
  var with_sea = with_map;
  let sea_opacity = overlay.sea_opacity * pole_mask;
  if (sea_opacity > 0.001) {
    let sea = textureSample(sea_tex, sea_sampler, input.uv_merc);
    let sea_mix = sea.a * sea_opacity;
    with_sea = mix(with_sea, sea.rgb, sea_mix);
  }
  var with_weather = with_sea;
  let weather_opacity = overlay.weather_opacity * pole_mask;
  if (weather_opacity > 0.001) {
    let weather = textureSample(weather_tex, weather_sampler, input.uv_merc);
    let weather_mix = weather.a * weather_opacity;
    with_weather = mix(with_weather, weather.rgb, weather_mix);
  }
  return vec4<f32>(with_weather, 1.0);
}
