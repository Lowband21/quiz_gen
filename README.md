# QuizGen: An AI-Powered Quiz Generation Tool

QuizGen is a versatile and powerful tool that utilizes artificial  intelligence to generate quizzes from multi-modal data. The project combines Rust and Python to preprocess data and generate quizzes, along with other functions.

- ## Prerequisites

To run QuizGen, you need to have the following software installed on your machine:

- Rust compiler (`rustc`) version 1.70 or higher
  - Use `rustup update` to update your Rust installation to the latest stable version

- FFmpeg
- Python 3 and pip
- PyTorch (torch)
  - Note: While PyTorch is automatically installed during the setup process, you might encounter issues on some systems
- clang

## Installation

Here are the steps to get QuizGen up and running on your system:

### Linux and macOS

1. Add your OpenAI API key to your environment variables:

```
export OPENAI_API_KEY=YOUR_API_KEY
```

2. Install Rust using the official one-liner:

```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

3. Install FFmpeg using your system's package manager.

4. Navigate to the `rusty_gen` directory and run the QuizGen with the following command:

`cargo run --release`

**Note:** In case the release build fails to compile, use the `cargo run` command for debugging.

### Windows

1. Add your OpenAI API key to your environment variables:

```export OPENAI_API_KEY=YOUR_API_KEY```

1. Install Rust using `rustup`.
2. Install Visual Studio with the C development dependencies.
3. Install FFmpeg.

**Notes for Windows Users:**

- When installing Rust with `rustup`, the program will offer to install Visual Studio. However, this will not include the C development dependencies, which you must install separately.
- Some features of QuizGen may not be fully implemented or tested on the Windows platform.

## Usage Notes

- The transcription operation will complete successfully even if there are no files in the input directory. This means that you will not receive an error message if your input directory is empty.
- The `download_script.sh` will download files named `index.html` and `index.html.1`. These files are not necessary for QuizGen's operation and should be deleted.

## PyTextbookProcessor

PyTextbookProcessor is a set of Python utilities included in QuizGen. It is designed to preprocess textbook data for the quiz generation process. The tool contains a variety of functions that make it easier to format and clean your data before generating quizzes.
