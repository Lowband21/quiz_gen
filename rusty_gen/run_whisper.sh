#!/bin/bash

#Check if pipenv is installed and install it if not
if ! command -v pipenv &> /dev/null
then
    echo "pipenv could not be found, installing it now"
    pip install pipenv
fi

#Set the directory where the script is located
DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"

#Navigate to that directory
cd $DIR
cd whisper

#Check if Pipfile exists, if not initialize pipenv
if [ ! -f Pipfile ]; then
    pipenv --python 3.11.3
fi

#Install the dependencies
pipenv install torch
pipenv install openai-whisper

#Run the script
pipenv run python whisper_all.py
