A simple WGSL vertex shader
lines=6 code=4 comments=1 blank=1
---
// A simple WGSL vertex shader

@vertex
fn main() -> @builtin(position) vec4<f32> {
  return vec4<f32>(0.0, 0.0, 0.0, 1.0);
}
