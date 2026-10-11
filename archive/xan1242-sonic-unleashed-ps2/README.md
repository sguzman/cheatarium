# Sonic Unleashed PS2 Hacks

Here are some of my hacks made for Unleashed PS2 during a quick session of screwing around.

## TODO:

- figure out time scaling issues for 60FPS

- port to other regions (currently only SLES-55380 PAL-E is supported)

## Host filesystem usage

1. Ensure that you have "Enable Host Filesystem" enabled in your PCSX2 settings (or `EmuCore/HostFs` enabled in the PCSX2.ini)

2. Extract all of the data from the ISO

3. Rename the game executable to `serialnumber.elf`, e.g. `SLES-55380.elf`

4. Move everything from PS2ROOT to the root directory (next to the ELF)

5. Launch the game by using "Start File" option and selecting the elf

6. Go to "Game Properties" and enable the cheat `Use Host Interface (HTCI)` (NOTE: if you do not see cheats - make sure your pnach filename has no serial number and ONLY the CRC, just like in the repo)

7. Reset

## NoAFS usage

1. Extract ONEPS2.afs using [AFSPacker](https://github.com/MaikelChan/AFSPacker/releases/latest) into a directory named `ONE_PS2` located next to the executable. 

2. The AFSPacker tool may produce files with names `pro`, `pro (1)`, `pro (2)`, etc. In this case, rename the files using a bulk file renaming tool. The filenames should be in format `pro_0000.one`, `pro_0001.one`, `pro_0002.one`, etc.

3. Enable the cheat `Read One Files Outside AFS (NoAFS) (By ID)` found in the Filesystem pnach.

4. You can now safely load the game without ONEPS2.afs
