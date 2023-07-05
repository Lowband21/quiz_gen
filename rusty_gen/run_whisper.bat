@echo off
REM First we need to check if pipenv is installed, if not, we need to install it
pipenv --version >nul 2>&1 || (
    echo pipenv could not be found, installing it now
    pip install pipenv
)

REM Set the directory where the script is located
set DIR=%~dp0
REM Navigate to that directory
cd /d %DIR%

REM Check if Pipfile exists, if not initialize pipenv
if not exist Pipfile (
    pipenv --python 3.11.3
)

REM Install the dependencies
pipenv install torch
pipenv install openai-whisper

REM Run the script
pipenv run python whisper_all.py
