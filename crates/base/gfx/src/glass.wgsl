// A pane of glass: the backdrop under it, already blurred, under the pane's tint, inside a
// rounded box. Nothing bends, magnifies or lights it.

@group(0) @binding(0) var blurred: texture_2d<f32>;
@group(0) @binding(1) var smp: sampler;

struct VsOut {
  @builtin(position) pos: vec4<f32>,
  @location(0) tint: vec4<f32>,
  @location(1) local: vec2<f32>,
  @location(2) half: vec2<f32>,
  @location(3) radius: f32,
  @location(4) screen: vec2<f32>,
};

@vertex
fn vs(
  @location(0) pos: vec2<f32>,
  @location(1) tint: vec4<f32>,
  @location(2) local: vec2<f32>,
  @location(3) half: vec2<f32>,
  @location(4) radius: f32,
  @location(5) screen: vec2<f32>,
) -> VsOut {
  var o: VsOut;
  o.pos = vec4<f32>(pos, 0.0, 1.0);
  o.tint = tint;
  o.local = local;
  o.half = half;
  o.radius = radius;
  o.screen = screen;
  return o;
}

// How far `p` stands outside a rounded box of half size `b` and radius `r`; negative inside.
fn outside(p: vec2<f32>, b: vec2<f32>, r: f32) -> f32 {
  let q = abs(p) - b + vec2<f32>(r);
  return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - r;
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
  let r = min(in.radius, min(in.half.x, in.half.y));
  let cover = clamp(0.5 - outside(in.local, in.half, r), 0.0, 1.0);
  if (cover <= 0.0) {
    discard;
  }
  let uv = clamp(in.pos.xy / in.screen, vec2<f32>(0.0), vec2<f32>(1.0));
  let under = textureSampleLevel(blurred, smp, uv, 0.0).rgb;
  let color = mix(under, in.tint.rgb, in.tint.a);
  return vec4<f32>(color, cover);
}
