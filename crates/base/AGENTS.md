# base — one way out of the process each

A base crate is a door: disk state, the network, a process, the GPU. It holds no
application logic and knows no domain type: it depends on no Groove crate, not even
`groove-types` or another base crate. A rule about the work belongs in the module that
calls it. Enforced by `every_dependency_points_down_the_layers`.

Test each crate standalone, against what stands behind its door: an in-memory database
for `db`, a mock server for `http` with redaction asserted on the wire, a scripted binary
for `exec`, golden images from `Frame` fixtures for `gfx`.

`gfx` asks wgpu for an adapter: CI gives it software Vulkan (lavapipe), a Mac gives it
Metal.
