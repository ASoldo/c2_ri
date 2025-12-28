struct Camera {
  view_proj: mat4x4<f32>,
};

struct TileUniform {
  radius: f32,
  opacity: f32,
  _pad: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;
@group(0) @binding(1)
var<uniform> tile: TileUniform;
@group(0) @binding(2)
var tile_tex: texture_2d_array<f32>;
@group(0) @binding(3)
var tile_sampler: sampler;

struct VertexInput {
  @location(0) uv: vec2<f32>,
  @location(1) bounds: vec4<f32>,
  @location(2) layer: f32,
};

struct VertexOutput {
  @builtin(position) clip_position: vec4<f32>,
  @location(0) uv: vec2<f32>,
  @location(1) @interpolate(flat) layer: f32,
};

fn to_radians(deg: f32) -> f32 {
  return deg * 0.01745329252;
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
  var out: VertexOutput;
  let lon = to_radians(mix(input.bounds.x, input.bounds.y, input.uv.x));
  let lat = to_radians(mix(input.bounds.w, input.bounds.z, input.uv.y));
  let cos_lat = cos(lat);
  let pos = vec3<f32>(
    tile.radius * cos_lat * cos(lon),
    tile.radius * sin(lat),
    tile.radius * cos_lat * sin(lon)
  );
  out.clip_position = camera.view_proj * vec4<f32>(pos, 1.0);
  out.uv = input.uv;
  out.layer = input.layer;
  return out;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
  let tex = textureSample(tile_tex, tile_sampler, input.uv, i32(input.layer));
  let alpha = tex.a * tile.opacity;
  if (alpha <= 0.001) {
    discard;
  }
  return vec4<f32>(tex.rgb, alpha);
}
