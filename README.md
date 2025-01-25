# Deep Cock Galactic
```
i dont think the name's solid atm. here are my thoughts on it:
    - Deep Stim Galactic (current repo name)
    - Deep Cock Galactic ("cock" sounds like "rock", idk)
    - Deep Throb Galactic (idk)
    - Deep Buzz Galactic (suggestion from the discord) (personally, i dont quite like it -flexxy)
~ flexxy

..also, hi sov!
```

(instructions are wip)

## Compilation
- cd into the `rust` dir
- cargo build that shit
- dll in `Target/` (i hope; idk how to make a windows .dll from linux rn)
- lua is lua, needs no compilation :p

## Installation

### using Auto script
```
would be cool if we could slap together a script to automatically put things in places or sth
~ flexxy
```

### manual
- compile this (see above)
- install [ue4ss](https://github.com/UE4SS-RE/RE-UE4SS) ([guide](https://docs.ue4ss.com/dev/installation-guide.html))
- [acquire lua54.dll](https://luabinaries.sourceforge.net/), and put that in the base game dir (eg. `.../steamapps/common/Deep Rock Galactic`)
- create the following dir: `{base game dir}/Mods/DeepStimGalactic/Scripts`
- copy `lua/main.lua` and `rust/Target/luabutt.so` into that new dir
- run game (i hope)
