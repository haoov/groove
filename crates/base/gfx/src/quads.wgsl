struct VsOut {
  @builtin(position) pos: vec4<f32>,
  @location(0) color: vec4<f32>,
  @location(1) local: vec2<f32>,
  @location(2) half: vec2<f32>,
  @location(3) shape: vec4<f32>,
  @location(4) lattice: f32,
};

@vertex
fn vs(
  @location(0) pos: vec2<f32>,
  @location(1) color: vec4<f32>,
  @location(2) local: vec2<f32>,
  @location(3) half: vec2<f32>,
  @location(4) shape: vec4<f32>,
  @location(5) lattice: f32,
) -> VsOut {
  var out: VsOut;
  out.pos = vec4<f32>(pos, 0.0, 1.0);
  out.color = color;
  out.local = local;
  out.half = half;
  out.shape = shape;
  out.lattice = lattice;
  return out;
}

// How far `p` stands outside a box of half size `b` whose corners round by `r`; inside is negative.
fn outside(p: vec2<f32>, b: vec2<f32>, r: f32) -> f32 {
  let q = abs(p) - b + vec2<f32>(r);
  return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - r;
}

// `v` brought into the cell of size `cell` centred on the origin.
fn wrapped(v: vec2<f32>, cell: vec2<f32>) -> vec2<f32> {
  return v - cell * floor(v / cell + vec2<f32>(0.5));
}

// How far `p` stands inside its hex, of corner radius `s`, pointy at the top, from the corner.
fn into_hex(p: vec2<f32>, s: f32) -> f32 {
  let w = s * 1.7320508;
  let cell = vec2<f32>(w, 3.0 * s);
  let a = wrapped(p - vec2<f32>(0.5 * w, s), cell);
  let b = wrapped(p - vec2<f32>(0.0, 2.5 * s), cell);
  let q = abs(select(b, a, dot(a, a) < dot(b, b)));
  return 0.5 * w - max(q.x, 0.5 * q.x + 0.8660254 * q.y);
}

// How much of a fade `blur` wide inside the rounded box this pixel keeps.
fn kept(in: VsOut, radius: f32, blur: f32) -> f32 {
  let inner = max(in.half - vec2<f32>(blur), vec2<f32>(0.0));
  let d = outside(in.local, inner, min(radius, min(inner.x, inner.y)));
  return 1.0 - smoothstep(-blur, blur, d);
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
  let radius = in.shape.x;
  let stroke = in.shape.y;
  let blur = in.shape.z;
  let hatch = in.shape.w;
  if (in.lattice > 0.0) {
    let width = 0.5 * max(stroke, 1.0) + 0.5;
    let edge = clamp(width - into_hex(in.pos.xy, in.lattice), 0.0, 1.0);
    let fade = select(1.0, kept(in, radius, blur), blur > 0.0);
    return vec4<f32>(in.color.rgb, in.color.a * edge * fade);
  }
  if (hatch > 0.0) {
    let along = in.pos.x + in.pos.y;
    let into = along - hatch * floor(along / hatch);
    return vec4<f32>(in.color.rgb, in.color.a * clamp(1.0 - into, 0.0, 1.0));
  }
  if (blur > 0.0) {
    return vec4<f32>(in.color.rgb, in.color.a * kept(in, radius, blur));
  }
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
