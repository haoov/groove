struct VsOut {
  @builtin(position) pos: vec4<f32>,
  @location(0) color: vec4<f32>,
  @location(1) local: vec2<f32>,
  @location(2) half: vec2<f32>,
  @location(3) shape: vec2<f32>,
};

@vertex
fn vs(
  @location(0) pos: vec2<f32>,
  @location(1) color: vec4<f32>,
  @location(2) local: vec2<f32>,
  @location(3) half: vec2<f32>,
  @location(4) shape: vec2<f32>,
) -> VsOut {
  var out: VsOut;
  out.pos = vec4<f32>(pos, 0.0, 1.0);
  out.color = color;
  out.local = local;
  out.half = half;
  out.shape = shape;
  return out;
}

// How far `p` stands outside a box of half size `b` whose corners round by `r`; inside is negative.
fn outside(p: vec2<f32>, b: vec2<f32>, r: f32) -> f32 {
  let q = abs(p) - b + vec2<f32>(r);
  return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - r;
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
  let radius = in.shape.x;
  let stroke = in.shape.y;
  if (radius <= 0.0 && stroke <= 0.0) {
    return in.color;
  }
  let d = outside(in.local, in.half, min(radius, min(in.half.x, in.half.y)));
  var cover = clamp(0.5 - d, 0.0, 1.0);
  if (stroke > 0.0) {
    cover = cover * clamp(0.5 + d + stroke, 0.0, 1.0);
  }
  return vec4<f32>(in.color.rgb, in.color.a * cover);
}
