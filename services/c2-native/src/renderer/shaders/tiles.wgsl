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

fn mercator_lat(merc: f32) -> f32 {
  let y = (0.5 - merc) * 6.28318530718;
  return 2.0 * atan(exp(y)) - 1.57079632679;
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
  var out: VertexOutput;
  let u = 1.0 - input.uv.x;
  let lon = to_radians(mix(input.bounds.x, input.bounds.y, u));
  let merc = mix(input.bounds.z, input.bounds.w, input.uv.y);
  let lat = mercator_lat(merc);
  let cos_lat = cos(lat);
  let pos = vec3<f32>(
    tile.radius * cos_lat * cos(lon),
    tile.radius * sin(lat),
    tile.radius * cos_lat * sin(lon)
  );
  out.clip_position = camera.view_proj * vec4<f32>(pos, 1.0);
  out.uv = vec2<f32>(u, input.uv.y);
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
