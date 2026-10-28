@group(0) @binding(0) var src: texture_2d<f32>;
@group(0) @binding(1) var smp: sampler;

struct VsOut {
  @builtin(position) pos: vec4<f32>,
  @location(0) uv: vec2<f32>,
};

// One triangle over the whole target.
@vertex
fn vs(@builtin(vertex_index) i: u32) -> VsOut {
  let xy = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
  var out: VsOut;
  out.pos = vec4<f32>(xy * 2.0 - 1.0, 0.0, 1.0);
  out.uv = vec2<f32>(xy.x, 1.0 - xy.y);
  return out;
}

fn at(uv: vec2<f32>) -> vec4<f32> {
  return textureSampleLevel(src, smp, uv, 0.0);
}

// Dual Kawase, down a level: the centre and its four diagonals.
@fragment
fn down(in: VsOut) -> @location(0) vec4<f32> {
  let o = 1.0 / vec2<f32>(textureDimensions(src));
  let d = vec2<f32>(o.x, -o.y);
  let sum = at(in.uv) * 4.0 + at(in.uv - o) + at(in.uv + o) + at(in.uv + d) + at(in.uv - d);
  return sum / 8.0;
}

// Dual Kawase, up a level: a ring of eight around the centre.
@fragment
fn up(in: VsOut) -> @location(0) vec4<f32> {
  let o = 1.0 / vec2<f32>(textureDimensions(src));
  var sum = at(in.uv + vec2<f32>(-2.0 * o.x, 0.0)) + at(in.uv + vec2<f32>(2.0 * o.x, 0.0));
  sum += at(in.uv + vec2<f32>(0.0, -2.0 * o.y)) + at(in.uv + vec2<f32>(0.0, 2.0 * o.y));
  sum += (at(in.uv + o) + at(in.uv - o) + at(in.uv + vec2<f32>(o.x, -o.y)) + at(in.uv + vec2<f32>(-o.x, o.y))) * 2.0;
  return sum / 12.0;
}

@fragment
fn copy(in: VsOut) -> @location(0) vec4<f32> {
  return textureLoad(src, vec2<i32>(in.pos.xy), 0);
}
