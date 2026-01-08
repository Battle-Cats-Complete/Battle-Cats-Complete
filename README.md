# Battle Cats Complete (Legacy)

[![Discord](https://img.shields.io/badge/Discord-Join%20Community-7289DA?style=for-the-badge&logo=discord&logoColor=white)](https://discord.gg/SNSE8HNhmP)

An all-in-one tool for The Battle Cats that allows Users to sort, read, and modify files that they import into it.

This is the old version before the codebase had a major modular overhaul, as I realized how huge the scope of this project really is.

This is the source repository of the app—for developers' eyes only. This version has "Game Decryption" logic in place of "Sorting Set-up" and "Zip Extraction" logic. The public repos' sanitized logic is within the "sanitized" folder.

## Push to Public
To push the project and its sanitized files to the [Public Repo](https://github.com/WonderMOMOCO/Battle-Cats-Complete):

1. Click the Actions tab at the top
2. On the left sidebar, click "Sync to Public Repo"
3. Click the "Run workflow" button on the right, select branch (main), and click the green button

## Usage
First, you must extract your game files using the Guide below, then select the folder containing them to sort them into the database to be read!

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

## Credit
Many people/groups have helped and encouraged me to make this project:
* TimTams, providing knowledge on most of the games csv logic
* TheWWRNerdGuy, inadvertantly convincing me to learn Rust and providing some csv logic
* SweetDonut0, providing a "source of truth" via working Javascript code to port/review from the [Miraheze Wiki](https://battlecats.miraheze.org/wiki/Battle_Cats_Wiki)
* [Battle Cats Ultimate](https://github.com/battlecatsultimate), an unoptimized and poorly maintained app that I use as a "proof of concept"

## Legal
This project is for educational purposes only. Assets are copyright PONOS Corp. Please support the official release.
