#!/bin/bash
# 用法：shots.sh <输出目录> <标签> state[:文件名] ...   每个状态单独一个 Edge 配置目录，避免丢图
SP="C:/Users/ADMINI~1/AppData/Local/Temp/claude/60c801a49415fe01/b35a5742-7dbc-4fe8-aa52-26645903e112/scratchpad"
OUT="$1"; TAG="$2"; shift 2
EDGE="/c/Program Files (x86)/Microsoft/Edge/Application/msedge.exe"
URL="file:///$SP/mockups/v4-A-title/index.html"
for spec in "$@"; do
  st="${spec%%:*}"; fn="${spec#*:}"; [ "$fn" = "$spec" ] && fn="preview-$st.png"
  rm -f "$OUT/$fn"
  "$EDGE" --headless=new --user-data-dir="$SP/edge_profile_v4_${TAG}_$st" --window-size=1200,720 --hide-scrollbars --virtual-time-budget=4000 --screenshot="$OUT/$fn" "$URL?state=$st" >/dev/null 2>&1 &
done
wait
for i in $(seq 1 40); do
  miss=0
  for spec in "$@"; do st="${spec%%:*}"; fn="${spec#*:}"; [ "$fn" = "$spec" ] && fn="preview-$st.png"; [ -s "$OUT/$fn" ] || miss=1; done
  [ $miss = 0 ] && break
  sleep 0.5
done
for spec in "$@"; do st="${spec%%:*}"; fn="${spec#*:}"; [ "$fn" = "$spec" ] && fn="preview-$st.png"; ls -la "$OUT/$fn" 2>&1 | awk '{print $5, $NF}'; done
