# Battle Cats Complete

[![Discord](https://img.shields.io/badge/Discord-Join%20Community-7289DA?style=for-the-badge&logo=discord&logoColor=white)](https://discord.gg/SNSE8HNhmP)

An all-in-one desktop toolkit for The Battle Cats. Load raw game data, load modded data, accurately render in-game animations, and export animations to formats like MP4, AVIF, WebP, and GIF.

This app is a hobby and passion project. Join the [Discord Server](https://discord.gg/SNSE8HNhmP) if you have any questions, suggestions, or if you found any bugs!

## Usage
To use this app, you must provide game files for it to read. These files are not included in the app or repository; they must be obtained through your own personal and legal means.

## Current Features
- **Importing Game Data**
  - Direct import from emulator/android
  - Import from `.pack` / `.apk` files
  - Import from standard archives
- **Import, View, Export Mods**
  - Import from Android, Packs, or Files
  - Add custom icons and metadata
  - Patch mods into the live database
  - Inject mod into APK/XAPK
- **Displaying Cat Data**
  - Icons and Banners
  - Forms, Stats, and Abilities
  - Talents and Evolution Info
  - In-game Descriptions
- **Displaying Enemy Data**
  - Icons, Stats, and Abilities
  - In-game Descriptions
- **Displaying Stage Data**
  - Battleground, Treasure, and Materials
  - Niche features like Fixed Line-Ups
  - Uses data/images from Cats & Enemies
- **Unit Animations**
  - View Walk, Idle, Attack, and Knockback
  - View Burrow, Surface, and Spirit animations
  - Raw Model viewing
- **Export Animations**
  - Export to GIF, WebP, AVIF, and more
  - Export single animations or custom sequences
  - User-defined camera area rendering
- **Advanced Filtering**
  - Filter Cats and Enemies by Forms, Rarity, or Talents
  - Search using specific Ability Toggles
  - Search using exact Ability Mathematical Attributes
- **Export Statblocks**
  - Generate statblocks for both Cats and Enemies
  - Instantly copy to clipboard
  - Export as a saved Image
  - Dynamically scaling image sizes


### Setup
Because this app is an open-source hobby project and not signed with a paid certificate, your OS may flag it on the first run.

> 🛡️ **Windows Users:**
> You may see a blue popup saying **"Windows protected your PC"** (Microsoft Defender SmartScreen).
>
> To start the app, click **More info**, then click **Run anyway**.

> 🍎 **MacOS Users:**
> You may see an error saying **"App is damaged and can't be opened"** or **"Unidentified Developer"** (Gatekeeper).
>
> To fix this, type the following command in Terminal: `xattr -cr`, then drag the app file into the terminal window to auto-fill the path.

BCC requires two additonal third-party resources to unlock full functionality. Without these, the apps capabilities are limited:
- Rooted Android device/emulator that is not Bluestacks and can run The Battle Cats
- Legally obtained Decryption Keys and Initialization Vectors for every region

## Credit
Various people have motivated and helped me create this project:
- **TheWWRNerdGuy:** Provided a repo which holds Rust code for reading raw game data, also gave coding tips.
- **SweetDonut0:** Used their Cat Stats Tool & Animation Viewer web-app hosted on the Wiki as a reference for code.
- **fieryhenry:** Coder with many Battle Cats related projects of their own, of which I took inspiration from.
- **Timtams:** Did field research to confirm certain game data quirks, also supplied some custom assets.

>This is an unofficial, educational tool. For full details regarding PONOS Corp. copyrights, authorized handling of game files and decryption keys, and our strict liability disclaimer, please read LEGAL.md before use.
