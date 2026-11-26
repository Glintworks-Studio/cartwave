#!/bin/bash
# MC-F1: menu key equivalents while a text field is edited (F-03 on macOS).
# Real key events; evidence = the saved project's clip count, the Edit menu's
# enabled states (AX) and a screenshot of the field.
source "$(dirname "$0")/lib.sh"
launch textmenu "$REPO/docs/songs/v5/showcase.chipdaw"
P="$PROJ"
drv click 1100 152 ";" key d cmd ";" sleep 0.3 ";" key s cmd ";" sleep 0.8
echo "after Cmd+D in the arrangement: $(clips $P) clips (8 + 1)"
# Cmd+Z in the search field: the song must keep the duplicate.
drv click 110 130 ";" sleep 0.2 ";" type abc ";" key z cmd ";" sleep 0.3
drv key escape ";" click 1300 800 ";" key s cmd ";" sleep 0.8
echo "after Cmd+Z in the field: $(clips $P) clips (want 9)"
# Cmd+A / C / V in the field act on the text, not on the clips.
drv click 1300 800 ";" key escape ";" sleep 0.2
echo "Duplicate before (nothing selected): $(drv menus | grep -E '^\s+Duplicate\b' | grep -q disabled && echo disabled || echo ENABLED)"
drv click 110 130 ";" sleep 0.2 ";" type abc ";" key a cmd ";" sleep 0.2
echo "Duplicate after Cmd+A in the field: $(drv menus | grep -E '^\s+Duplicate\b' | grep -q disabled && echo disabled '(no clips selected: right)' || echo ENABLED '(clips selected: wrong)')"
drv type q ";" key a cmd ";" key c cmd ";" key right ";" key v cmd ";" sleep 0.3 ";" shot "$MC_WORK/textmenu-field.png"
echo "Paste after Cmd+C in the field: $(drv menus | grep -E '^\s+Paste\b' | grep -q disabled && echo disabled '(no clips copied: right)' || echo ENABLED '(clips copied: wrong)')"
drv key escape ";" click 1300 800 ";" key s cmd ";" sleep 0.8
echo "after Cmd+V in the field: $(clips $P) clips (want 9)"
echo "field screenshot: $MC_WORK/textmenu-field.png (want 'qq')"
# A dialog open: Cmd+Z / Cmd+D don't reach the song behind it (fresh launch).
quit
launch textmenu2 "$REPO/docs/songs/v5/showcase.chipdaw"
P="$PROJ"
drv click 1100 152 ";" key d cmd ";" sleep 0.3 ";" key s cmd ";" sleep 0.8
echo "dialog test, after Cmd+D: $(clips $P) clips (9)"
drv key , cmd ";" sleep 0.5 ";" shot "$MC_WORK/textmenu-settings.png" ";" key z cmd ";" key d cmd ";" key d cmd ";" sleep 0.3 ";" key escape ";" sleep 0.3 ";" key s cmd ";" sleep 0.8
echo "after Cmd+Z, Cmd+D, Cmd+D with Settings open: $(clips $P) clips (want 9)"
quit
