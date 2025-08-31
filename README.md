# veritas

[![veritas](https://img.shields.io/badge/veritas-Discord-%235865F2.svg)](https://discord.gg/Y9kSnPk95H) [![GitHub Releases](https://img.shields.io/github/downloads/hessiser/veritas/total.svg)]()

**Veritas**, the magnum opus, of a mundane-chalk-throwing pillar man from Hirohiko Araki's critically acclaimed series Honkai Star Rail. It is powered by rust and egui and is a damage logger/damage tracker/damage meter/ACT.

This is a fork of the original [hessiser/veritas](https://github.com/hessiser/veritas) repository, with additional features and quality-of-life changes.

👉 For base installation and usage, please consult the original [Wiki](https://github.com/hessiser/veritas/wiki). The key changes in this version are listed below.

---
## ✨ Modifications in This Version

* **Simplified DLL Naming:**
    The core file has been renamed from `veritas.dll` to **`xluau.dll`**. This is to make the process of copying the file for injection easier and more convenient.

* **Improved Battle Mode Detection:**
    The logic for detecting battle modes has been enhanced for greater accuracy, please copy `battle_modes.json` to your game installation folder.

* **📈 Automatic Battle Analysis (JSON Summaries & Analyzer Script)**
    This version introduces a powerful workflow for analyzing your battle performance.

    **Step 1: Automatic Data Generation**
    The tool automatically generates a detailed **JSON summary** for each battle session. These files contain rich data about your team's performance.
    * **Location:** The summaries are saved in a `battle_summaries` folder, which is created inside the game's client directory (the same location as the game's `.exe` file).

    **Step 2: Visualize and Compare with `analyzer.py`**
    Included in this project is an `analyzer.py` script, a tool to parse, visualize, and compare the data from your battle summaries. 
    
    [Click here to view a screenshot of the analyzer](img/analyzer.jpg)

    To use it, follow these instructions:

    1.  **Find your data:** Navigate to your game installation folder and locate the `battle_summaries` directory.
    2.  **Copy the folder:** Copy the entire `battle_summaries` folder and paste it into the same directory where the `analyzer.py` script is located.
    3.  **Install dependencies:** Open a terminal or command prompt in the script's directory and run:
        ```bash
        pip install -r requirements.txt
        ```
    4.  **Run the analyzer:** Once the dependencies are installed, run the script with:
        ```bash
        python analyzer.py
        ```
    A graphical window will open, displaying charts and comparisons of your battle sessions.

---
## Supported Targets

- Windows

---
## Maintainers

### Fork Maintainer
- [leovn85](https://github.com/leovn85/) (Feature additions)

### Original Maintainers
- [Hessiser](https://github.com/hessiser/) (Backend/UI/Misc)
- [NightKoneko](https://github.com/NightKoneko/) (UI/Misc)
- [Nushen](https://github.com/NuShen1337/) (Backend/Il2Cpp)
