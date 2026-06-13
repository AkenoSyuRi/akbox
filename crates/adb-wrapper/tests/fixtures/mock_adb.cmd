@echo off
if "%1"=="devices" if "%2"=="-l" (
  echo List of devices attached
  echo AAA111       device product:p1 model:Phone1 device:d1 transport_id:1
  echo BBB222       device product:p2 model:Phone2 device:d2 transport_id:2
  exit /b 0
)
exit /b 0