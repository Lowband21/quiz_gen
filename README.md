# Quiz Generation
## Dependencies:
- `rustc` >= 1.70
    - Use `rustup update` to update to the latest stable toolchain
- `ffmpeg`
- `python` and `pip`
- `torch` 
    - Automatically installed but issues may arise


### Install
#### Linux and MacOS

1. Add OpenAI API key to environment
   `export OPENAI_API_KEY=YOUR_API_KEY`
2. Install rustup using the official one-liner
   `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
3. Install ffmpeg
4. Run `cargo run --release` in the rusty_gen directory

> - I've had some issues with the the release build, so use `cargo run` as a debugging step if it fails to compile

#### Windows

1. Add OpenAI API key to environment
   `export OPENAI_API_KEY=YOUR_API_KEY`
2. Install `rustup`
3. Install visual studio with C development dependencies
4. Install `ffmpeg`

> Notes: 
>
> - `rustup` will automatically offer to install visual studio, but it will not install the optional C development dependencies
>
> - Certain functionalities have not been implemented or tested on the Windows platform
### Usage Notes
- Transcription will exit successfully if there are no files in the input directory
- `download_script.sh` will also download `index.html` and `index.html.1`, these should be deleted

## PyTextbookProcessor
A collection of preprocessor utilities for textbook data, written in Python.
