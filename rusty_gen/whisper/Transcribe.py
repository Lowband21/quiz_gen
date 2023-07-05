#@markdown Click here if you'd like to save the transcription as text file
plain = True #@param {type:"boolean"}

#@markdown Click here if you'd like to save the transcription as an SRT file
srt = False #@param {type:"boolean"}

#@markdown Click here if you'd like to save the transcription as a VTT file
vtt = False #@param {type:"boolean"}

#@markdown Click here if you'd like to save the transcription as a TSV file
tsv = False #@param {type:"boolean"}

#@markdown Click here if you'd like to download the transcribed file(s) locally
download = False #@param {type:"boolean"}

import os, re
import torch
from pathlib import Path
from pytube import YouTube

import whisper
from whisper.utils import get_writer

# %%
# Use CUDA, if available
DEVICE = "cuda" if torch.cuda.is_available() else "cpu"

# Load the desired model
model = whisper.load_model("medium.en").to(DEVICE)

# %% [markdown]
# ## 💪 YouTube helper functions
# 
# Code for helper functions when running Whisper on a YouTube video.

# %%
def to_snake_case(name):
    return name.lower().replace(" ", "_").replace(":", "_").replace("__", "_")

# %% [markdown]
# # ✍ Transcribing with Whisper
# 
# Ultimately, calling Whisper is as easy as one line!
# * `result = model.transcribe(file)`
# 
# The majority of this new `transcribe_file` function is actually just for exporting the results of the transcription as a text, VTT, or SRT file.

# %%
def transcribe_file(model, file, plain, srt, vtt, tsv, download):
    """
    Runs Whisper on an audio file

    Parameters
    ----------
    model: Whisper
        The Whisper model instance.
    
    file: str
        The file path of the file to be transcribed.

    plain: bool
        Whether to save the transcription as a text file or not.
    
    srt: bool
        Whether to save the transcription as an SRT file or not.
    
    vtt: bool
        Whether to save the transcription as a VTT file or not.
    
    tsv: bool
        Whether to save the transcription as a TSV file or not.

    download: bool
        Whether to download the transcribed file(s) or not.

    Returns
    -------
    A dictionary containing the resulting text ("text") and segment-level details ("segments"), and
    the spoken language ("language"), which is detected when `decode_options["language"]` is None.
    """
    file_path = dir + file
    print(f"Transcribing file: {file_path}\n")

    output_directory = file_path

    # Run Whisper
    result = model.transcribe(file_path, verbose = False, language = "en")

    if plain:
        # txt_path = file_path.with_suffix(".txt")
        
        audio_path = dir + "Audios/" + file
        text_path =  dir + "Transcriptions/" + file[:-4] + ".txt" # Replace the  extension with .txt

        print(f"\nCreating text file")
        
        with open(text_path, "w", encoding="utf-8") as txt:
            txt.write(result["text"])

        os.rename(file_path, audio_path)
    return result

# %% [markdown]
# # 💬 Whisper it!
# 
# This block actually calls `transcribe_file` 😉
# 

# %%
dir = "/home/lowband/dev/quiz_gen/rusty_gen/transcriptions_in"

# %%
folder = os.listdir(dir)

# Loop through the audio files and transcribe them
# for folder in dir:
for audio_file in folder:
  print(audio_file)
  # Extract the audio from the video file using librosa
  # file = dir + audio_file
  # Skip the file if it is not a video format
  #if not audio_file.endswith((".mp4", ".3gp")):
  #  continue


  # Run Whisper on the specified file
  result = transcribe_file(model, audio_file, plain, srt, vtt, tsv, download)

  print("Result: " + result)
  


