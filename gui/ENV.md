# Environment Variables

- `BCC_FORCE_RESCAN={N}`: Pretends the caches are out of date, so you get the same thing users see right after an update. Old data shows up instantly, a rescan runs in the background, fresh data swaps in. `{N}` can be anything. The rescan saves under that token, so running again with the same value starts warm; bump it (`1` -> `2`) or drop the variable to trigger another one.

- `WGPU_BACKEND={backends}`: Picks which graphics backends wgpu may use, comma separated — `vulkan`, `dx12`, `metal`, `gl`, `webgpu`. Windows defaults to `dx12,gl` because enumerating Vulkan drags in every overlay's layer (Steam, Discord, etc.) and cost 5-20 seconds at launch. Set it yourself and the app leaves your choice alone, which makes it handy for timing comparisons.
