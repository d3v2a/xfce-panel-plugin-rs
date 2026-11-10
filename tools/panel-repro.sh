#!/usr/bin/env bash
# Hermetic xfce4-panel harness: Xvfb + private D-Bus session + isolated XDG dirs.
# Loads the workspace plugins into a throwaway panel and reports the panel's exit
# code (139 = SIGSEGV), so a plugin crash can be reproduced without touching the
# developer session.
#
# usage: tools/panel-repro.sh <label> [iterations]
#   PLUGINS="demo hello"   plugins to load, from target/$PROFILE
#   PROFILE=debug          cargo profile to take the .so from
#   OVERWRITE=none         none | cp | rename: replace libdemo.so while the panel
#                          has it mapped, then quit. cp (same inode) rewrites the
#                          code pages the panel is executing -> SIGSEGV in the save
#                          path; rename (new inode) is safe.
set -u
label="${1:-run}"
iters="${2:-1}"
PLUGINS="${PLUGINS:-demo hello}"
PROFILE="${PROFILE:-debug}"
OVERWRITE="${OVERWRITE:-none}"
root="$(cd "$(dirname "$0")/.." && pwd)"
libs="$root/target/repro/libs"
v2="$root/target/repro/v2"

rm -rf "$libs" "$v2"; mkdir -p "$libs" "$v2"
for p in $PLUGINS; do
  cp "$root/target/$PROFILE/lib$p.so" "$libs/" || exit 1
  d="$(find "$root/target/$PROFILE/build" -name "$p.desktop" -print -quit)"
  [ -n "$d" ] && cp "$d" "$libs/" || { echo "no .desktop for $p" >&2; exit 1; }
done
if [ "$OVERWRITE" != none ]; then
  v2src="${V2SO:-$root/target/release/libdemo.so}"
  [ -f "$v2src" ] || { echo "need a different demo image: cargo build --release -p xfce-panel-demo (or set V2SO)" >&2; exit 1; }
  cp "$v2src" "$v2/libdemo-v2.so"
fi

for i in $(seq 1 "$iters"); do
  out="$root/target/repro/$label-$i"
  rm -rf "$out"
  mkdir -p "$out/datahome/share/xfce4/panel/plugins" \
           "$out/datahome/lib/xfce4/panel/plugins" \
           "$out/cfg/xfce4/xfconf/xfce-perchannel-xml" \
           "$out/cache" "$out/home"
  cp "$libs"/*.desktop "$out/datahome/share/xfce4/panel/plugins/"
  cp "$libs"/*.so "$out/datahome/lib/xfce4/panel/plugins/"
  {
    echo '<?xml version="1.0" encoding="UTF-8"?>'
    echo '<channel name="xfce4-panel" version="1.0">'
    echo '  <property name="configver" type="int" value="2"/>'
    echo '  <property name="panels" type="array">'
    echo '    <value type="int" value="1"/>'
    echo '    <property name="panel-1" type="empty">'
    echo '      <property name="position" type="string" value="p=6;x=0;y=0"/>'
    echo '      <property name="length" type="double" value="100"/>'
    echo '      <property name="position-locked" type="bool" value="true"/>'
    echo '      <property name="size" type="uint" value="32"/>'
    echo '      <property name="plugin-ids" type="array">'
    id=0
    for p in $PLUGINS; do id=$((id + 1)); echo "        <value type=\"int\" value=\"$id\"/>"; done
    echo '      </property>'
    echo '    </property>'
    echo '  </property>'
    echo '  <property name="plugins" type="array">'
    id=0
    for p in $PLUGINS; do id=$((id + 1)); echo "    <value type=\"int\" value=\"$id\"/>"; done
    id=0
    for p in $PLUGINS; do
      id=$((id + 1))
      echo "    <property name=\"plugin-$id\" type=\"string\" value=\"$p\"/>"
    done
    echo '  </property>'
    echo '</channel>'
  } > "$out/cfg/xfce4/xfconf/xfce-perchannel-xml/xfce4-panel.xml"
  env -i PATH="$PATH" HOME="$out/home" DISPLAY=":$((89 + i))" NO_AT_BRIDGE=1 \
      XDG_CONFIG_HOME="$out/cfg" XDG_CACHE_HOME="$out/cache" XDG_DATA_HOME="$out/datahome/share" \
      PANEL_DEBUG=module,application,main,module-factory \
      dbus-run-session -- bash -c '
        Xvfb "$DISPLAY" -screen 0 1280x1024x24 >/dev/null 2>&1 &
        xvfb=$!
        sleep 2
        xfce4-panel --disable-wm-check > "'"$out"'/panel.log" 2>&1 &
        panel=$!
        sleep 12
        so="'"$out"'/datahome/lib/xfce4/panel/plugins/libdemo.so"
        if [ "'"$OVERWRITE"'" != none ]; then
          echo "inode before: $(stat -c %i "$so")"
          if [ "'"$OVERWRITE"'" = cp ]; then
            cp "'"$v2"'/libdemo-v2.so" "$so" 2>&1 | head -2
          else
            cp "'"$v2"'/libdemo-v2.so" "$so.new" && mv -f "$so.new" "$so"
          fi
          echo "inode after: $(stat -c %i "$so")"
          sleep 3
        fi
        xfce4-panel -q 2>&1 | sed -e "s/\x1b\[[0-9;]*m//g"
        sleep 4
        if kill -0 "$panel" 2>/dev/null; then alive=alive; kill -KILL "$panel"; else alive=gone; fi
        wait "$panel"; prc=$?
        kill "$xvfb" 2>/dev/null
        echo "panel_exit=$prc alive=$alive"
      ' > "$out/result.txt" 2>&1
  pat="$(echo "$PLUGINS" | tr -s ' ' '|')"
  echo "== $label iter=$i: $(grep -E 'panel_exit' "$out/result.txt")"
  echo "   loaded=$(grep -c 'new item' "$out/panel.log") plugins=$(grep -cE "name=($pat)" "$out/panel.log") saved=$(grep -c 'finalized' "$out/panel.log")"
done
