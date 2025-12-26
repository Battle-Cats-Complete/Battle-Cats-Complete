# Battle Cats Complete

A high-performance tool written in Rust to decrypt, extract, and animate assets from *The Battle Cats*.

## Features
Currently implemented:
* **Decrypt Game Files:** Multi-threaded extraction of `.pack` and `.list` files (Supports JP, EN, TW, KR).
* **Automatic Detection:** Automatically detects region and encryption type (standard or server-style).
* **Extracts All Languages:**  Supports all in-game languages from the Global version that other programs opt to skip.
* **Sorts Game Data:** Game data is stored in a simple, human-readable file structure for easy user access.

## Roadmap
I plan to implement these features soon:
* [ ] Read & Sort Cat Data
* [ ] Read & Sort Enemy Data
* [ ] Data Language Priority
* [ ] Animation Player
* [ ] Animation to AVIF Export
* [ ] Read & Sort Stage Data

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

### 5. Decrypt
1. Open **Battle Cats Complete**.
2. Navigate to the "Import Data" window
2. Click "Select Game Folder".
3. Choose the folder where you saved the files. The tool will automatically decrypt, extract, and sort everything into a file structure inside the `/game` folder.

*This process will have to be repeated every time the game updates to obtain new content.*

## Disclaimer
This project is for educational purposes only. Assets are copyright PONOS Corp. Please support the official release.