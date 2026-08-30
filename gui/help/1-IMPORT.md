# Import
You Import game files through the `Import` page. This page can both import game files and sort existing files on disk.

## Obtaining Files
There are three methods for obtaining game files: `Android`, `Pack`, and `Raw`.

### Android
Pull game files from the installed app on an android device or emulator. This method requires the following:
- **Root Access** to the device you are pulling the game from so BCC can access all of its files.
- **Android Bridge** downloaded through `Settings > Addons > Android Bridge` so the BCC has access to ADB.
- **ADB Debugging** on your device so BCC can see and pull files using ADB.
- **Keys & IV** added through `Settings > General > Manage Keys` so BCC can decrypt the games `.pack` file format.

If you are interfacing with a real android device using Windows, you may need to download the device's associated OEM under `Settings > Addons > OEM` or by finding it yourself through a search engine.

Known supported emulators are **MuMuPlayer** and **LDPlayer**. Emulators that I cannot support include **BlueStacks**.

### Pack
Decrypt `.pack` files provided to BCC. This method requires the following:
- **Encrypted `.pack` Files** for BCC to decrypt for raw game files.
- **Keys & IV** added through `Settings > General > Manage Keys` so BCC can decrypt the games `.pack` file format.

### Raw
Copy raw game files you provide to BCC. This method requires the following:
- **Raw Game Files** for BCC to copy into its database.

Alternatively, you can create the `game` folder next to BCC's binary and move the files there yourself.

## Sorting Files
There are two methods for sorting an existing database, using the `Raw` import method targeting specifically `game` or `game/raw`:

### `game`
Targeting `game` with `Raw` import will flatten the entire database into `raw` and then proceed to run the database sorter on the now flattened database.

### `game/raw`
targeting `game/raw` with `Raw` import will skip the flattening, only moving unsorted files within `raw` to their new locations.