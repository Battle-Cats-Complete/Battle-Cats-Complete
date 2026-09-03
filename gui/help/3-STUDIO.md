# Studio
The `Studio` page edits an entity's animation files directly: its sprite sheet (`.png`), its cut list (`.imgcut`), its model (`.mamodel`), and its animations (`.maanim`). Every change is written to disk as you make it.

**Disclaimer:** Studio writes files in place and has no save step, only a per-set 25-change-history undo. Back up a set before working on it.

## Sets
A **set** is the group of files that make up one entity. Sets live in the `studio` folder, one folder per set. A set under `studio` is edited in place.

Files from anywhere else are copied before the first edit, never written where they sit:
- **Mod Enabled:** the files are copied into that Mod, then edited there.
- **No Mod, `game` Unlocked:** the files are edited in place. Unlock through `Settings > Files > Editor`.
- **Anything Else:** the files are copied into `studio` under the set's name.

Renaming a set renames its folder, and is only allowed while the set is in `studio`.

### Manage
`Manage` handles the files in a set.
- **Import Set** copies an existing entity's files in from the database.
- **New Set** creates an empty set to build from scratch.
- The folder list picks any set already in `studio`.
- **PNG**, **IMGCUT** and **MAMODEL** replace a single file. A loaded file shows green with its name.
- **Add MAANIM** and **Remove MAANIM** manage the set's animations.
- **Open Folder** reveals the set on disk, and is only available for sets in `studio`.

`Export` zips the loaded set into the `exports` folder.

## Atlas
`Atlas` edits the sprite sheet's cut list. `Add Cut` appends a new region.
- **Set:** Allows you to right-click and drag to set the bounds of that cut.
- **Trim:** Resize the cut to remove any dead space.
- **Find:** Centers your camera on the part.
- **Select:** Selects the part, making its row entry blue and outline bold.

You can also Select parts by clicking on them in the Atlas viewer.

## Entity
`Entity` edits the model and the selected animation together. The left tree lists the model's parts; expanding a part lists the **channels** animating it. Selecting either fills the table on the right.

A **channel** is one animated property of one part, such as its angle or opacity, holding the keyframes that drive it. A part may carry two channels of the same kind, in which case the earlier one is marked `overridden`, as only the last one takes effect.

Right-clicking a part offers to add a part beneath it, add or remove a channel, or delete it.

`View` jumps the playhead to a keyframe. `Bound` loops playback across the segment that keyframe begins.

## Colors
The viewer draws its overlays in a fixed set of colors. Which ones appear is toggled in the `Option` column.
- **Red** outlines parts. Every part is outlined faintly; the selected one is outlined boldly.
- **Cyan** marks origins. Each part gets a dot at the point it rotates and scales around, and the selected part is joined to its parent's origin by a line.
- **Yellow** is the selected part's own direction, running from its origin out through the top of the part.
- **Purple** is The Hand. Enables live entity edits. It brightens while you work with it.
- **Green** is the world rather than any part: the ground line, the entity's height, and a mark at the world's origin.

## The Hand
Parts can be posed directly in the viewer. Left click selects a part and reveals it in the tree; left click it again to deselect. Right-click pauses the viewer and allows you to right-click drag on certain areas of the selected part to pose it:
- **Middle** moves the part.
- **Edges & Corners** stretch it. Dragging an edge past the opposite one flips it.
- **The Ring** on the part's axis rotates it.
- **Scrolling** while holding changes its opacity.

The `Option` column picks what the hand writes. This is disabled and forced to `ModelHand` when no animation is loaded.

### ChannelHand
Writes to the selected animation, keyframing the current frame only. If the part has no channel for the property, one is created. This is the default option.

### ModelHand
Writes to the model's rest pose, which affects every frame of every animation.

## Undo
`Ctrl+Z` reverts the last change, up to 25 back. History is kept per set, and is discarded when a different set is loaded.
