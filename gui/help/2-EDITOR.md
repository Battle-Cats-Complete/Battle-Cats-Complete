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

Where the viewer normally offers **Export**, the Editor offers **Sync "game"**, which replaces the animation you are editing with the game's own copy. It asks once before doing it, and is unavailable when your Mod has no copy of that file to replace.

### Action
**Locate** centres the view on the part your selected curve drives.

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

### Keyframes
Selecting a curve fills the table below the viewer with its keyframes. The row tinted blue is the one currently driving the animation, and it moves as the animation plays.

Each row carries two shortcuts. **View** pauses playback and jumps to that keyframe. **Bound** sets the viewer's frame range to that keyframe's segment, stopping one frame before the next keyframe so the following segment never plays. The last keyframe bounds to itself, holding the pose it ends on.

The **Curve** column sets how a keyframe eases into the next one.

A curve that repeats forever never highlights its last keyframe. That keyframe is where the curve wraps back to its first, so the game never rests on it.

A part showing `No children or curves` is one this animation does not touch. Every animation of a Unit shares one model, so a part built for the attack sits unused in the walk.

A part marked `not drawn` or `no sprite` is never drawn at all. One marked `transparent` or `no scale` is only hidden where it rests, and an Opacity or Scale curve can bring it in partway through.
