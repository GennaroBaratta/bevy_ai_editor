@echo off
cd /d D:\workspace\bevy_ai_editor\examples\simple_game
title Simple Game (Bevy 0.18)
echo ==========================================
echo Starting Simple Game Host...
echo ==========================================
cargo run
if %ERRORLEVEL% NEQ 0 (
    echo.
    echo Error occurred!
    pause
)
