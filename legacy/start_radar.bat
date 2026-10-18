@echo off
title Kurdistan SkyRadar - Live Air Traffic
echo ========================================================
echo       KURDISTAN SKYRADAR - LIVE FLIGHT TRACKER
echo ========================================================
echo Starting local telemetry engine...
start http://localhost:8080
python server.py
pause
