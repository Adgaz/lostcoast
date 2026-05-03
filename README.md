# lostcoast

Trying to rebuild the Half-Life 2 / Source 1 look in Rust on Vulkan. Mostly to learn how the old radiosity bake + RNM thing works.

## what works

- [x] vulkan instance, device, swapchain on macOS via MoltenVK
- [x] clear color, present
- [x] hello triangle, push constants, dynamic rendering
- [x] textured cube, vertex / index / uniform buffers, depth, fly camera (WASD + mouse)

## what doesn't yet

- [ ] glTF loader, scene graph, MSAA
- [ ] real time directional light, half-Lambert, normal mapping
- [ ] offline lightbaker (direct only)
- [ ] full radiosity bake with multi bounce indirect
- [ ] RNM lightmaps (3 textures composed by the bump normal in the shader)
- [ ] ambient cube probes for dynamic models
- [ ] HDR pipeline (fp16, compute autoexposure histogram, bloom, linear tonemap)
- [ ] env cubemaps with parallax correction
- [ ] dynamic point and spot lights, Phong + Fresnel, PCF shadow maps

## run

```sh
cargo run --bin app -- --scene assets/scenes/cube.json
```

WASD to move, mouse to look, click to grab, escape to release.

## scope

The math is the Mitchell, McTaggart, Green 2006 SIGGRAPH course notes. It will not match a HL2 screenshot exactly because the look also depends on Valve's content tuning (VRAD parameters, artist authored materials, exposure curves) which I'm not trying to reproduce.

Also not doing: BSP, MDL, VTF, VMT parsing. glTF + a small JSON sidecar for lightmap params, lights, env probes.

## license

MIT or Apache 2.0.
