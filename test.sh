#!/usr/bin/env bash
set -x

pkill Xephyr

Xephyr :1 -ac -br -noreset -screen 1280x720 +extension RANDR &
DISPLAY=:1 cargo r --bin nwm
