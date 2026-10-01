#!/bin/sh
set-e

REPO=""
DEST="$HOME/YUP"

echo "== Downloading dependecies =="
apk update
apk add vim cargo git build-base

echo "Cloning repository"

cd "$DEST"
if [-d "$DEST/.git"];then
  git -C "$DEST" pull
else
  git clone "$REPO" "$DEST"
fi

echo "== Compiling =="

cd "$DEST"
cargo build --release

echo "== READY, Opening =="

./target/release/Roulette
