struct Camera {
  view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;
@group(0) @binding(1)
var icon_tex: texture_2d_array<f32>;
@group(0) @binding(2)
var icon_sampler: sampler;

struct VertexInput {
  @location(0) position: vec2<f32>,
  @location(1) uv: vec2<f32>,
  @location(2) instance_pos: vec3<f32>,
  @location(3) instance_size: f32,
  @location(4) instance_color: vec4<f32>,
  @location(5) instance_heading: f32,
  @location(6) instance_kind: u32,
};

struct VertexOutput {
  @builtin(position) clip_position: vec4<f32>,
  @location(0) uv: vec2<f32>,
  @location(1) color: vec4<f32>,
  @location(2) @interpolate(flat) kind: u32,
};

fn safe_east(normal: vec3<f32>) -> vec3<f32> {
  let up = vec3<f32>(0.0, 1.0, 0.0);
  var east = cross(up, normal);
  if (dot(east, east) < 1e-6) {
    east = cross(vec3<f32>(0.0, 0.0, 1.0), normal);
  }
  return normalize(east);
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
  let normal = normalize(input.instance_pos);
  let east = safe_east(normal);
  let north = normalize(cross(normal, east));
  let cos_h = cos(input.instance_heading);
  let sin_h = sin(input.instance_heading);
  let up = normalize(north * cos_h + east * sin_h);
  let right = normalize(cross(up, normal));
  let offset = right * (input.position.x * input.instance_size)
    + up * (input.position.y * input.instance_size);
  let world = input.instance_pos + offset;

  var out: VertexOutput;
  out.clip_position = camera.view_proj * vec4<f32>(world, 1.0);
  out.uv = input.uv;
  out.color = input.instance_color;
  out.kind = input.instance_kind;
  return out;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
  let tex = textureSample(icon_tex, icon_sampler, input.uv, i32(input.kind));
  return tex * input.color;
}
