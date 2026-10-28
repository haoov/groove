// The lens is ported to WGSL and changed from Kyant0/AndroidLiquidGlass, Apache-2.0:
// see LICENSE-AndroidLiquidGlass.txt at the crate root. Its rounded-rect refraction shader.

@group(0) @binding(0) var blurred: texture_2d<f32>;
@group(0) @binding(1) var smp: sampler;

struct VsOut {
  @builtin(position) pos: vec4<f32>,
  @location(0) tint: vec4<f32>,
  @location(1) local: vec2<f32>,
  @location(2) half: vec2<f32>,
  @location(3) shape: vec2<f32>,
  @location(4) screen: vec2<f32>,
};

@vertex
fn vs(
  @location(0) pos: vec2<f32>,
  @location(1) tint: vec4<f32>,
  @location(2) local: vec2<f32>,
  @location(3) half: vec2<f32>,
  @location(4) shape: vec2<f32>,
  @location(5) screen: vec2<f32>,
) -> VsOut {
  var o: VsOut;
  o.pos = vec4<f32>(pos, 0.0, 1.0);
  o.tint = tint;
  o.local = local;
  o.half = half;
  o.shape = shape;
  o.screen = screen;
  return o;
}

// How far in the outer edge reads the backdrop, in rim widths.
const AMOUNT: f32 = 2.0;
// How much the pull leans toward the centre, so corners bend round.
const DEPTH: f32 = 1.0;
const CHROMA: f32 = 1.0;
// How much the whole pane magnifies at its centre, fading to none at the rim.
const BULGE: f32 = 0.25;
const SATURATION: f32 = 1.5;
// How far the backdrop moves toward black under a dark tint, and toward white under a light one.
const DARKEN: f32 = 0.55;
const LIGHTEN: f32 = 0.08;
const RIM: f32 = 0.1;
const SHADOW: f32 = 0.3;

fn outside(p: vec2<f32>, b: vec2<f32>, r: f32) -> f32 {
  let q = abs(p) - b + vec2<f32>(r);
  return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - r;
}

// The outward normal of a rounded box of half size `b` and radius `r` at `p`.
fn normal(p: vec2<f32>, b: vec2<f32>, r: f32) -> vec2<f32> {
  let c = abs(p) - (b - vec2<f32>(r));
  if (c.x >= 0.0 || c.y >= 0.0) {
    return sign(p) * normalize(max(c, vec2<f32>(0.0)) + vec2<f32>(0.0001));
  }
  let gx = step(c.y, c.x);
  return sign(p) * vec2<f32>(gx, 1.0 - gx);
}

fn backdrop(at: vec2<f32>, screen: vec2<f32>) -> vec3<f32> {
  let uv = clamp(at / screen, vec2<f32>(0.0), vec2<f32>(1.0));
  return textureSampleLevel(blurred, smp, uv, 0.0).rgb;
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
  let small = min(in.half.x, in.half.y);
  let r = min(in.shape.x, small);
  let height = max(min(in.shape.y, small), 1.0);
  let sd = outside(in.local, in.half, r);

  let dropped = outside(in.local - vec2<f32>(0.0, height * 0.2), in.half, r);
  let shadow = SHADOW * pow(1.0 - smoothstep(-height * 0.2, height * 0.6, dropped), 2.0);
  let cover = clamp(0.5 - sd, 0.0, 1.0);
  if (cover <= 0.0) {
    return vec4<f32>(0.0, 0.0, 0.0, shadow);
  }

  let round = min(r * 1.5, small);
  let x = clamp(1.0 + min(sd, 0.0) / height, 0.0, 1.0);
  let pull = (1.0 - sqrt(max(1.0 - x * x, 0.0))) * height * AMOUNT;
  let toward = normalize(in.local + vec2<f32>(0.0001));
  let grad = normalize(normal(in.local, in.half, round) + DEPTH * toward);
  let shift = -pull * grad;
  let spread = CHROMA * (in.local.x * in.local.y) / (in.half.x * in.half.y);
  let u = abs(in.local / in.half);
  let q = pow(u.x * u.x * u.x * u.x + u.y * u.y * u.y * u.y, 0.25);
  let dome = 1.0 - min(q * q, 1.0);
  let at = in.pos.xy + shift - in.local * BULGE * dome * dome;
  var color = vec3<f32>(
    backdrop(at + shift * spread, in.screen).r,
    backdrop(at, in.screen).g,
    backdrop(at - shift * spread, in.screen).b,
  );
  let luma = dot(color, vec3<f32>(0.213, 0.715, 0.072));
  color = max(mix(vec3<f32>(luma), color, SATURATION), vec3<f32>(0.0));
  let dark = dot(in.tint.rgb, vec3<f32>(0.213, 0.715, 0.072)) < 0.5;
  color = select(mix(color, vec3<f32>(1.0), LIGHTEN), color * (1.0 - DARKEN), dark);
  color = mix(color, in.tint.rgb, in.tint.a);

  // A thin rim, lit on the top-left and the bottom-right edges.
  let line = smoothstep(-1.5, -0.5, sd);
  let lit = abs(dot(normal(in.local, in.half, round), vec2<f32>(-0.7071, -0.7071)));
  color += vec3<f32>(RIM * line * lit);

  let alpha = cover + shadow * (1.0 - cover);
  return vec4<f32>(color * cover / max(alpha, 0.0001), alpha);
}
