# NFSC PS3 Patches

This is a quick and dirty collection of patches for NFS Carbon PS3.

# Instructions

## Requirements
- Windows
- Python 3
- A decrypted EBOOT.BIN as EBOOT.ELF (placed in the `bin` directory) (you can use tools such as TrueAncestor SELF/EBOOT Resigner)

## Compatibility

Currently only compatible with `BLUS30016`.

## Steps

1. Concatenate all of the codes together into the `patches.txt` file (should be located right next to `patchnbuild.bat`) (NOTE: any patches that go into a different section go into its own text file, will be noted in the cheat)
2. Run `patchnbuild.bat`
3. Copy `EBOOT-patched.BIN` in place of the original `EBOOT.BIN` in your game (either in ISO or the PS3 HDD, both should work).

NOTE: `EBOOT-patched.BIN` is built as an FSELF, so if you want to build it as a proper SELF, the ELF file is generated right next to it should you need it.

## Example of what a patches.txt should look like

```
00245374 38600001
00245378 4E800020
000FC0F0 38600001
000FF8AC 38600001
004D09E8 38000000
0041359C 38600001
004135A0 8122A598
004135A4 98690000
00663680 60000000
```

## Patch List (BLUS30016)

[Go here](patches/BLUS30016/Patches.md)

# Where are the RPCS3 patches?

Will be done in the future...
