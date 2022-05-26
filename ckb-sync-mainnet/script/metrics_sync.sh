#!/bin/bash

ckb_dir="$1"
ckb_sync_target_number=-1
file_name="$ckb_dir"/metrics-$(date "+%Y%m%d")
reach_end=false
while ! $reach_end; do
  # every 10 minutes, save current_tip and shared_best_header into file
  sleep 600
  cur_tip_str=$(curl -X POST -H 'Content-Type: application/json' \
    -d '{ "id": 42, "jsonrpc": "2.0", "method":"get_tip_block_number", "params":[] }' \
    http://localhost:8114 | grep result | gawk -F '"' '{ print strtonum($8) }')
  cur_tip=$((cur_tip_str))

  best_header=$(curl -X POST -H 'Content-Type: application/json' \
    -d '{ "id": 42, "jsonrpc": "2.0", "method":"sync_state", "params":[] }' \
    http://localhost:8114 | grep best_known_block_number | awk -F '"' '{ print strtonum($10) }')
  now=$(date "+%Y-%m-%dT%H:%M:%S")
  echo "$now|$cur_tip|$best_header" >>"$file_name"

  if [ $cur_tip -eq $ckb_sync_target_number ]; then
    reach_end=true
  else
    ckb_sync_target_number=$cur_tip
    reach_end=false
  fi
done

# read/paring file and plot line graph
"$ckb_dir"/sync-chart "$file_name"