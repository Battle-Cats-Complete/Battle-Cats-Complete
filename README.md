# Battle Cats Complete (Legacy)
This is the old version of Battle Cats Complete, before the codebase had a major modular overhaul due to the scope. This version is depricated and is here to provide a "source of truth" for any logic I may have left out of my refactored version. This version also acts as a guide for dumping the game, as I cant be bothered to handle sanitization with private and public versions of files anymore.

## Guide: Extracting Game Files

To use this tool, you need the encrypted game files from an Android device. I personally use [MuMu Player](https://www.mumuplayer.com/download/), but this method works on any rooted emulator or phone.

### 1. Install the Game
Download your desired region of *The Battle Cats*. You can use the Play Store or QooApp:
* **JP:** [Play Store](https://play.google.com/store/apps/details?id=jp.co.ponos.battlecats) | [QooApp](https://m-apps.qoo-app.com/app/1545)
* **EN:** [Play Store](https://play.google.com/store/apps/details?id=jp.co.ponos.battlecatsen) | [QooApp](https://m-apps.qoo-app.com/app/6730)
* **TW:** [Play Store](https://play.google.com/store/apps/details?id=jp.co.ponos.battlecatstw) | [QooApp](https://m-apps.qoo-app.com/app/6598)
* **KR:** [Play Store](https://play.google.com/store/apps/details?id=jp.co.ponos.battlecatskr) | [QooApp](https://m-apps.qoo-app.com/app/6817)

Your `[PACKAGE_NAME]` will vary depending on which region you downloaded:
* **JP:** `jp.co.ponos.battlecats`
* **EN:** `jp.co.ponos.battlecatsen`
* **TW:** `jp.co.ponos.battlecatstw`
* **KR:** `jp.co.ponos.battlecatskr`

*Open the game to download game data. Keep the app around to download future updates.*

### 2. Locate the Files
1. Download and open [MT Manager](https://mt-manager.en.softonic.com/android). Grant it **Root (Superuser)** permissions.
2. On the right panel, open a shared folder (e.g., your PC shared folder or Downloads).
3. On the left panel, navigate to **Root** (hit `..` until you reach the top).

### 3. Copy the Data
Navigate to the data folder for your specific region:
* **Path:** `/data/data/[PACKAGE_NAME]/files/`
* Copy the entire `files` folder to your shared folder.

### 4. Copy the APK
Navigate to `/data/app/`. Look for a folder containing `[PACKAGE_NAME]`.
* Inside, find `split_InstallPack.apk`.
* Copy this file to your shared folder.

*Note: If the `files` folder or `split_InstallPack.apk` file alredy exists and you are having trouble with updating them via overwriting, you can zip the files folder and copy it over, unzipping on your computer to guarantee you have new files*

### 5. Decrypt
1. Open **Battle Cats Complete**.
2. Navigate to the "Import Data" window
2. Click "Select Game Folder".
3. Choose the folder where you saved the files. The tool will automatically decrypt, extract, and sort everything into a file structure inside the `/game` folder.

*This process will have to be repeated every time the game updates to obtain new content.*

## Legal
This project is for educational purposes only. Assets are copyright PONOS Corp. Please support the official release.
