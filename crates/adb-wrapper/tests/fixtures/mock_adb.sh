#!/bin/sh
if [ "$1" = "devices" ] && [ "$2" = "-l" ]; then
  echo "List of devices attached"
  echo "AAA111       device product:p1 model:Phone1 device:d1 transport_id:1"
  echo "BBB222       device product:p2 model:Phone2 device:d2 transport_id:2"
  exit 0
fi
exit 0