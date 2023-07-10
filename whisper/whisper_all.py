import os
import torch
from pathlib import Path

import whisper

DEVICE = "cuda" if torch.cuda.is_available() else "cpu"
model = whisper.load_model("medium.en").to(DEVICE)

def transcribe_file(model, file):
    dir = os.getcwd()
    inf = "/input/"
    outf  = "/output/"
    in_path = dir + inf + file
    print(f"Transcribing file: {in_path}\n")

    result = model.transcribe(in_path, verbose = False, language = "en")

    out_audio_path = dir + outf + "Audios/" + file
    out_text_path =  dir + outf + "Transcriptions/" + file[:-4] + ".txt"
    print(f"\nCreating text file")

    with open(out_text_path, "w", encoding="utf-8") as txt:
        txt.write(result["text"])

    os.rename(in_path, out_audio_path)
    return result

for file in os.listdir("./input"):
    transcribe_file(model, file)
