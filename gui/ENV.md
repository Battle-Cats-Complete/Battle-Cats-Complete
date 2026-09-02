# Environment Variables

- `FORCE_RESCAN={true|false}`: Pretends the caches are out of date, so you get the same thing users see right after an update. Old data shows up instantly, a rescan runs in the background, fresh data swaps in. Set it to `true` and **every** run rescans. Anything else, or leaving it unset, behaves normally and starts warm. Handy for exercising the post-update path repeatedly, since that is the only time the index is rebuilt.

- `WGPU_BACKEND={backends}`: Picks which graphics backends wgpu may use, comma separated: `vulkan`, `dx12`, `metal`, `gl`, `webgpu`. Windows defaults to `dx12,gl` because enumerating Vulkan drags in every overlay's layer (Steam, Discord, etc.) and cost 5-20 seconds at launch. Set it yourself and the app leaves your choice alone, which makes it handy for timing comparisons.
