# Deep Stim Galactic!

# :3
:3

## warning: still very much in-development
like, you can install it ig, it has a hotkey that makes your buttplug vibrate. but like, thats all..

## this mod relies on buttplug.io and intiface to actually control your buttplug.
specifically, this mod acts as a client communicating with a Buttplug server.

you'll wanna install [Intiface Central](https://intiface.com/central). follow the instructions there for setup.


## how to install
TODO: actually make a release that can be downloaded in Step 3

1. install [UE4SS](https://github.com/UE4SS-RE/RE-UE4SS) ([guide](https://docs.ue4ss.com/dev/installation-guide.html))
2. download [the newest release of this mod](https://github.com/buttplug-drg/deep-stim-galactic/releases/latest) from the Releases section
3. unzip the zip; move things to the following places:
    - place the `Deepcock` directory with its contents into the UE4SS mod directory
    - place `lua54.dll` in the executable directory (`Deep Rock Galactic/FSD/Binaries/Win64/`; the directory that contains `FSD-Win64-Shipping.exe`, where you also placed `dwmapi.dll` while installing UE4SS)
    - add line `Deepcock : 1` to `mods.txt` to actually enable the mod

## if you're a dev
you will need the following:
- rustup
- cargo (can be acquired using rustup ig)
- unless using windows: cargo-cross (`cargo install cross`; used for cross compilation)
- a comfortable way to edit lua <inclusive or\> rust
- make sure you `rustup default stable` (idk, cargo-cross says to do this)

developing on any platform other than linux is currently untested and unsupported. should be simple enough tho

### build the buttplug bindings:
if you arent on windows:
- run `make build` (`make build-release` for an optimized non-debug build)
- if that doesnt work, do `cd rust` and then `cross build --target x86_64-pc-windows-gnu` (or `cargo cross build --target x86_64-pc-windows-gnu` maybe) (add `--release` flag to the command for optimizeed release build)

if you are on windows for whatever fucking reason:
- just `cd rust` and then `cargo build`, or `cargo build --release`. congratulations, you dont need to deal with cross-compilation.

either way, resulting artefact is `rust/target/x86_64-pc-windows-gnu/{release | debug}/luabutt.dll`

### build lua
jk, lua doesnt need to be "built" :p nothing needs to be done.

### symlinking files into your drg install
at this point, if your system is capable of symlinking, i highly recommend running `./link.sh debug /path/to/your/drg/install` (replace "debug" with "release" to link release binary instead) to symlink files to the appropriate places in your DRG install, to avoid having to copy them over every time you change anything.

if youre on windows... well, lmk if you find a good solution ig, or just draft a PR to add it. (someone implementing `copy.ps1` would be highly appreciated)

and dont forget to `Deepcock : 1` in `mods.txt` uwu

## install DSG the "hard" way
in your DRG install:
- make sure UE4SS is installed (see regular install guide above)
- make sure lua54.dll is in the executable directory (again, see above)
- go into the Mods directory
- make the following dir structure:
  ```
  Mods/  ((this is the Mods/ dir that youre currently in; dont actually create this inside of Mods/))
   └ Deepcock/
    └ Scripts/
  ```
- ...and place the contents of `lua/`, as well as `luabutt.dll` (from this repository) into `Scripts`.
- dont forget `Deepcock : 1` in `mods.txt`!!

but if you're already cloning the repo anyways, honestly, just symlink that shit. unless youre on windows.

damn windows users, i feel bad for you guys, your operating system Sucks qwq

# this mod loves trans ppl and furries, and was made by a trans catgirl, nya~ 🏳️‍⚧️ 🐾 ⚧️ 🐈‍⬛ 🦊 ❤️
if you have a problem with that, you do not deserve this mod uwu

if you install it anyways, i will personally break and enter into your home to remove this mod from your computer ^w^
