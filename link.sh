#!/usr/bin/env bash

ARTEFACTNAME="luabutt.dll"
LUADIR="$(realpath "./lua")"
MODNAME="Deepcock"

ARTEFACTDIR="$(realpath "rust/target/x86_64-pc-windows-gnu")"
if [[ "$1" = "debug" ]]; then
    ARTEFACTDIR="$ARTEFACTDIR/debug"
elif [[ "$1" = "release" ]]; then
    ARTEFACTDIR="$ARTEFACTDIR/release"
    exit 1
else
    echo "Invalid build specifier \"$1\""
    echo 'Valid specifiers: "debug", "release"'
    exit 1
fi

if [[ "$DRG_BASEDIR" = "" ]]; then
    if [[ "$2" = "" ]]; then
        echo "Must provide DRG base dir if environment variable \$DRG_BASEDIR is unset"
        exit 1
    fi
    DRG_BASEDIR="$2"
fi

DRG_BASEDIR="$(realpath "$DRG_BASEDIR")"
DRG_SCRIPTDIR="Scripts"

if [ -d "$DRG_BASEDIR" ]; then
    DRG_EXEDIR="$(realpath "$DRG_BASEDIR/FSD/Binaries/Win64")"
    DRG_UE4SSDIR="$DRG_EXEDIR"
    if [[ $UE4SS_EXPERIMENTAL = "1" ]]; then
        DRG_UE4SSDIR="$DRG_UE4SSDIR/ue4ss"
    fi
    DRG_MODDIR="$DRG_UE4SSDIR/Mods/$MODNAME"

    ln -sf "$ARTEFACTDIR/$ARTEFACTNAME" "$DRG_MODDIR/$ARTEFACTNAME"
    ln -sf "$LUADIR" "$DRG_MODDIR/$DRG_SCRIPTDIR"
    echo "Don't forget to put lua54.dll in the executable dir ($DRG_EXEDIR)"
else
    echo "Provided DRG base dir ($DRG_BASEDIR) is not a directory"
    exit 1
fi

