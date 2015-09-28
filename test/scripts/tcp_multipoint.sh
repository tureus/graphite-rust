#!/usr/bin/env sh
set -xe

BODY=$( cat <<EOF
local.random.diceroll1 4 `date +%s`
local.random.diceroll2 4 `date +%s`
local.random.diceroll3 4 `date +%s`
EOF
)

echo "$BODY" | nc -4 -w0 localhost 2003
