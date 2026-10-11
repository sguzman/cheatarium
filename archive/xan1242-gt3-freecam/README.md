# Gran Turismo 3 Free Camera Tool
This is a quick 'n' dirty tool made to allow for looking around the scenery in the game.

I'm not planning to polish this much further beyond what is right now (for the time being, due to other projects).

## USAGE
1. Run the GT3 FreeCam tool
2. Select your PCSX2 executable (or simply pass the path to PCSX2's executable as the first argument)
3. Run Gran Turismo 3 v1.00 NTSC-U

NOTE 1: If you run into a case where it says it failed to get the base address, simply try again. It should work.

NOTE 2: It will work with save states, but only with the save states that were made after this tool has patched the game!

NOTE 3: If you want to disable the UI or disable car rendering, you can use the cheats that are added as an extra in this repo.

## CONTROLS
### Keyboard
- Toggle camera: Backspace
- Movement: WASD
- Look: IJKL
- Move Up/Down: Space/Left Control
- Turbo/Super Turbo: Left Shift / F
- Tilt Counter-clockwise: U
- Tilt Clockwise: O
- Reset All Rotations: R

### Controller
NOTE: This currently only works on XInput. You need to have a controller connected to port 2.

Controls are similar to Black Box's debug camera controls.

- Toggle camera: Back/View (Select/Share on PS controllers)
- Movement: Left Stick & face buttons
- Look: Right Stick & D-pad
- Move Up/Down: RT/LT
- Turbo/Super Turbo: LB/RB
- Tilt Counter-clockwise: Start + LT
- Tilt Clockwise: Start + RT
- Reset All Rotations: Start + B

## LIMITATIONS
- This tool currently only modifies the view matrix with little to no information on the world matrix. In other words, the tool doesn't know the directions of anything within the world.
- The FPU calculations are slightly off compared to the (emulated) PS2. Because of this, the camera can look a little tilted after looking around. 

You can press the R (or Start + B) button to reset all of the camera rotations.

You can mitigate the tilting/rolling effect slightly by setting the rounding mode to "Nearest" and clamping mode to "Normal" on VU0.

- Try to avoid looking up/down and only look left/right to avoid any weirdness regarding tilting.
- Controllers are read **independently** of PCSX2. They're read on the PC side via XInput (on port 2).
- The controls are currently not configurable. You need to edit and recompile the app until that is added.
- This tool has no GUI. Everything is presented via the console window.
- It's currently relying exclusively on Win32 API. A Linux version could be done (but would likely need root access).

## COMPATIBILITY
This is currently only compatible with Gran Turismo 3 NTSC-U v1.00 (SCUS-97012)

Theoretically, this should be portable to other regions and other derivatives (older/newer versions).

This should be achievable by sigscanning.

## How does this work?
This tool simply acts as a "trainer" for PCSX2.

It consists of 2 parts: PS2 code patches and PC side memory manipulation

On the PS2 side - it firstly patches the game code to expose the pointer for the camera matrix at a known memory location. This is done by caving some extra code hooks into the empty/unused areas in the executable.

Then, it also adds a toggle switch to the camera matrix updater. This has to be done on the PS2 side because the code is JIT-ed/recompiled and then cached on PCSX2, so patching the code during runtime won't work (unless the recompiler cache is reset).

And lastly, as a simple and easy checkpoint when the camera should stop existing, I've added a check against the destructor of the "RaceCourse" object. This only happens when it's exiting an active race.

On the PC side - the tool accesses the PCSX2 VM memory simply by accessing a known pointer off of the PCSX2's base address.

These memory pages are marked as R/W, so no memory page permission editing is needed.

Using the known pointers that we made on the PS2 side, we can directly access the view matrix (of the OnBoardCamera object) and edit it. The game reads it in realtime.

## Could this be made to run on a real PS2?
If you fancy doing assembly with the VUs, then yes.

Otherwise - you'd need to find a way how to inject a pre-compiled ELF file into CORE.GT3 and edit the game's memory map accordingly.

Perhaps there are enough needed high-level functions already in the game to accomplish this. In that case - you theoretically could pull this off using only MIPS assembly. There is enough slack space in the executable for it.
