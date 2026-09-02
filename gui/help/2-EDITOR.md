# Editor
You can invoke the Editor using the **Context Menu**, allowing you to modify and create modded files for a currently enabled Mod. Mods are created through `Mods > Add Mod`, and are enabled through `Mods > YourModHere > Enable Mod`, where `YourModHere` is the name of the mod you'd like to enable.

**Disclaimer:** The Editor is currently an opt-in **Nightly** feature enabled through `Settings > General > Behavior > Enable Nightly Features`. This means it is potentially unstable and may be buggy. The Editor handles the creation, deletion, and writing of files. It is recommended you back-up your Mod's files before using the Editor while it is in this state.

## Context Menu
To select files you want to work with, you must right-click to open the Context Menu. You can select a specific file to handle, and select an action to take with it. There are two different options driving Context Menu file access under `Settings > Files > Context Scope`: `Broad` and `Specific`.

### Broad
Gives the Context Menu access to all currently loaded files related to the page you are seeing on your screen. Makes access easy and quick, but can feel bloated or confusing due to an abundant amount of options the Context Menu provides upfront. The default option.

### Specific
Gives the Context Menu access to files related only to what you right-clicked on. Allows you to decide what to edit by scanning the UI instead of scanning a verbose Context Menu list for your specific file. Requires basic knowledge of what files are used where in BCC for an efficient workflow.

## Editing Mode
There are two editing modes: `Raw` and `Resolved`. The mode can be changed under `Settings > Files > Editor > Editor Mode`.

### Raw
The real raw file values, with no fancy UI, options, or normalization. It's essentially a text editor that attaches a name to every column and/or row.

### Resolved
Abstracts away from raw file values to provide an easy-to-use but slightly more restrictive editing experience. It displays flags as buttons, clamps input to prevent unreasonable or impossible values, and transforms magic numbers into human-readable words and selections. The default option.

## Buffer Character
When editing a clamped field, you may sometimes notice that BCC resolves the value mid-write, making it difficult to write the value you want as it transforms inputs such as `100` into `200` when you start writing the value at `1`.

Numeric input fields accept a buffer character: `!`. When this character is included at the start of a field, your value will not be resolved or written until the field is "unfocused." Fields become "unfocused" when you click anywhere outside the input field.

## Animation
Right-clicking a Unit's animation opens the Animation Editor for that Unit's rig. The rest of the Context Menu still offers the usual per-file actions for the rig's own files. The Editor takes over the window, and the red `×` closes it.

Where the viewer normally offers **Export**, the Editor offers **Sync "game"**, which replaces the Unit's whole rig — its `.mamodel` and every one of its animations — with the game's own copies. It asks once before doing it, and is unavailable when your Mod has no copy of any of them.

It restores all of them together on purpose. Animations address parts by position, so a model and its animations only agree as a set; putting one file back on its own is how you end up with curves driving the wrong parts.

### Modes
The box in the Editor's top right switches what the panel edits, and names the file it is editing to its left. The viewer, the part tree and the tables under it never move between modes; the panel's lower half changes, and so does what the leftmost table below the viewer reports.

- **Animation** edits the selected clip's `.maanim` keyframes.
- **Model** edits the rig's `.mamodel` rest pose.
- **Atlas** replaces the Unit's sprite sheet and edits its cut list.

In `Animation` that table reports the selected part's resting **Model** values. In `Model` it reports the **Atlas** instead: which region of the sheet the part draws, where that region sits, how much of it is actually painted, and how much of it is transparent padding — `Margin` reads left, top, right, bottom.

`Atlas` mode has no tables under the viewer at all. It gives that space to the sheet itself, and swaps the animation for an image viewer — drag to pan, wheel to zoom, the same one the Utilities and Files pages use.

### Atlas
Three buttons load files into the Unit's rig. **The file you pick keeps nothing but its contents** — feeding `apple.png` to `030_f` gives you `030_f.png` holding apple's image, so the rig still finds it.

- **Atlas** takes a `.png` or an `.imgcut`, drops the extension, and loads both files of that name from the same folder.
- **Sheet** takes a `.png` and replaces the sprite sheet alone.
- **Cuts** takes an `.imgcut` and replaces the cut list alone.

Below them, every region of the cut list gets a row: its number, its position and size, its name, and three buttons.

- **Set** arms the viewer, exactly as `Set Camera` works in the animation exporter: the sheet dims the moment it is armed, the hint slides down, and right click and drag across it to draw the region. The region's own numbers are blanked and its outline hidden while you are redrawing it. A single right click, or a drag too small to be one, cancels and puts them back. A region may be drawn past the edge of the sheet.
- **Trim** shrinks a region to the pixels actually painted inside it, dropping the transparent border.
- **Find** centres the viewer on that region.
- **Select** picks the region out, and picks it again to let it go.

Clicking inside a region on the sheet selects it too — where regions overlap, the highest numbered one wins. The selected row is tinted and its region is drawn thicker than the rest. Only one at a time.

The sheet's own edge is outlined in green. A region reaching past it still draws, and **only the number actually at fault** turns red — hovering it says so. The game does not draw a part whose region falls outside the sheet.

**Removing a region renumbers the ones after it**, so the model's `Sprite` fields and every `Sprite` curve of every animation are rewritten to match. It asks once, and the last region cannot be removed.

### Model
Picking a part in the tree fills the table with the thirteen numbers the `.mamodel` gives that part, plus its name. The `#` column is the part's column number in the file itself. Changes reach the viewer as soon as they are written.

An empty cell means the value the game starts a fresh part at, which is what the greyed hint shows. Four fields are held to what the game can actually read, and typing past the edge lands on it: **Parent** stops at `-1` and at the last part, **Sprite** at `-1` and at the last region of the atlas, **Opacity** between nothing and full, and **Glow** across the four blending modes. Everything else is free.

The box beside the two numbers picks which **root offset** you are editing. These are the rows at the bottom of the `.mamodel`, and they place the whole Unit rather than any one part — the same list the viewer's offset picker chooses between, so `Root 0` is the one combat uses. The two numbers are the offset the Unit is placed by, subtracted rather than added, and measured against the root part's own pivot.

### Rearranging Parts
Parts can be added, removed and moved, in either mode.

- **Right-click a part** for `New "Part n"`, which adds a part under the one you clicked, and `Remove "Part n"`, which asks once. Right-clicking anywhere else offers `New` at the root.
- **Drag a part** by holding the left mouse button on its row for a moment, until a faded copy of the row lifts under the cursor. Dropping it on the middle of another part makes it that part's child. Dropping in the gap between two rows puts it in that gap: if the row below the gap is a child of the row above, it joins them as another child, and otherwise it becomes a sibling of the row above. The wheel still scrolls the tree while you drag. A part cannot be dropped inside itself. A shorter press is just a click, and selects.

A new part starts drawing immediately, using the Unit's own id, the first region of the atlas, and one layer in front of whatever it hangs off.

**Dragging only changes what a part hangs off. It never renumbers anything**, so no animation is touched and a part keeps the number it has always had.

**Removing a part does renumber**, because everything after it moves up one, and every animation of that Unit is rewritten to match. Curves that pointed at the removed part are dropped, since they no longer point at anything. That is why it asks first, and why it is the one action in the Editor that writes to files you did not open.

Where a part sits in the list is not what decides draw order — **Z Order** is. The list order only breaks ties between parts sharing a depth, which is why moving a part around in the file would buy nothing.

### Action
**Locate** centres the view on the part your selected curve drives, or in Model mode on the part you have picked.

### Adding and Removing Curves
Right-clicking a part or one of its curves offers to add a curve for any property that part does not already drive, or to remove one it does. A new curve starts at the value the game treats as no change, which is zero for most properties but the part's own parent, and full scale and opacity, for those.

### Part Overlay
Three buttons control the overlay, blue when on and grey when off. They stack, so a part lit by more than one shows bolder.

- **Rig:** every part the game is drawing, dimly.
- **Selected:** the part your selected curve drives, boldly.
- **Hierarchy:** that part boldly, plus its direct children.
- **Origin:** a green dot at the point the Unit is placed against.
- **World:** green guides along the ground line and up the Unit's height, spanning only the Unit itself. The upward guide never runs downward, so a Unit dipping below the ground line is showing you a mistake in its animation.

A bold part is drawn as a **bright red box** with a **cyan dot** at its anchor, a **yellow line** showing which way it faces, and a **cyan line** to the anchor of the part it hangs off. A dim part is a **faint red box** and a **faint cyan dot**.

The anchor is the point a part pivots around, not the middle of its box.

A part missing from the overlay is one the game is not drawing. The panel names the reason when you select one of its curves.

A part marked `not in the loaded model` is one the file has but the loaded rig does not, which means the two have drifted apart — reopening the Unit resettles it.

### Keyframes
Selecting a curve fills the table below the viewer with its keyframes. The row tinted blue is the one currently driving the animation, and it moves as the animation plays.

Every numeric field in the Editor takes the buffer character described above — keyframes, `Loop`, a part's rest pose, the root offsets and the atlas regions. It is worth using on **Frame**: keyframes stay in frame order, so committing a frame moves its row.

Each row carries two shortcuts. **View** pauses playback and jumps to that keyframe. **Bound** sets the viewer's frame range to that keyframe's segment, stopping one frame before the next keyframe so the following segment never plays. The last keyframe bounds to itself, holding the pose it ends on.

The **Curve** column sets how a keyframe eases into the next one. **Power** applies only to `Exponential`, where it bends the motion: negative values start fast and slow down, positive values start slow and speed up, and a larger number bends it harder. It is greyed out on the other curves, which ignore it.

A curve that repeats forever never highlights its last keyframe. That keyframe is where the curve wraps back to its first, so the game never rests on it.

A part showing `No children or curves` is one this animation does not touch. Every animation of a Unit shares one model, so a part built for the attack sits unused in the walk.

A part marked `not drawn` or `no sprite` is never drawn at all. One marked `transparent` or `no scale` is only hidden where it rests, and an Opacity or Scale curve can bring it in partway through.
