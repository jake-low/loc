A simple HLSL vertex shader
lines=5 code=3 comments=1 blank=1
---
// A simple HLSL vertex shader

float4 VSMain(float4 pos : POSITION) : SV_POSITION {
  return pos;
}
