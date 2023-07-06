use hound::WavReader;
use std::fs::File;
use std::path::Path;
use std::process::Command;

use std::env;
use std::fs;
use std::io::prelude::*;

use num_cpus;

use whisper_rs::{FullParams, SamplingStrategy, WhisperContext};

pub fn transcribe_all(model_size: String) {
    //let path_to_model = env::args().nth(1).unwrap();

    // Load a context and model
    if !Path::new(format!("ggml-{}.bin", model_size).as_str()).exists() {
        let bash_script_path = "download_model.sh";
        match Command::new("bash")
            .arg(bash_script_path)
            .arg(model_size.clone())
            .status()
        {
            Ok(status) => println!("Exited with status: {}", status),
            Err(err) => println!("Failed to execute script: {}", err),
        }
    }
    let ctx = WhisperContext::new(format!("ggml-{}.bin", model_size).as_str())
        .expect("failed to load model");

    // Create directory if it doesn't exist
    match fs::create_dir_all("whisper/input") {
        Ok(_) => println!("Input directory created successfully."),
        Err(err) => println!("Error creating input directory: {}", err),
    }

    // Get all the files in the input directory
    let entries = fs::read_dir("whisper/input").unwrap();

    for entry in entries {
        let entry = entry.unwrap();
        let audio_file = entry.path().to_str().unwrap().to_string();
        // Create a params object
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

        let num_threads = num_cpus::get();
        println!("Setting number of cpu threads to {}", num_threads);
        params.set_n_threads(num_threads.try_into().unwrap());

        // Read the audio file into a buffer (assuming it's a 32-bit floating point sample rate)
        let audio_data: Vec<f32> = read_audio_file(&audio_file);

        // Run the model
        let mut state = ctx.create_state().expect("failed to create state");
        state
            .full(params, &audio_data[..])
            .expect("failed to run model");

        // Fetch the results
        let num_segments = state
            .full_n_segments()
            .expect("failed to get number of segments");

        let mut result = String::new();

        for i in 0..num_segments {
            let segment = state
                .full_get_segment_text(i)
                .expect("failed to get segment");
            let start_timestamp = state
                .full_get_segment_t0(i)
                .expect("failed to get segment start timestamp");
            let end_timestamp = state
                .full_get_segment_t1(i)
                .expect("failed to get segment end timestamp");

            //result += &format!("[{} - {}]: {}\n", start_timestamp, end_timestamp, segment);
            result += &format!("{}\n", segment);
        }

        // Create directory if it doesn't exist
        match fs::create_dir_all("whisper/output") {
            Ok(_) => println!("Output directory created successfully."),
            Err(err) => println!("Error creating output directory: {}", err),
        }

        // Write the result to a file in the output directory
        let output_file = format!(
            "./whisper/output/{}.txt",
            entry.file_name().to_str().unwrap()
        );
        let mut file = File::create(output_file).expect("unable to create file");
        file.write_all(result.as_bytes()).expect("unable to write");
    }
}

fn read_audio_file(file_path: &str) -> Vec<f32> {
    let path = Path::new(file_path);
    let extension = path.extension().and_then(std::ffi::OsStr::to_str);

    let audio_path = match extension {
        Some("mp4") | Some("avi") | Some("mkv") => {
            // For video files, extract the audio track first
            let audio_path = path.with_extension("wav");
            let output = Command::new("ffmpeg")
                .arg("-i")
                .arg(file_path)
                .arg("-y") // Overwrite output
                .arg("-vn") // No video
                .arg("-acodec") // Audio codec
                .arg("pcm_s16le") // PCM 16 bit little endian format (compatible with WavReader)
                .arg("-ar") // Audio sampling rate
                .arg("16000") // 16KHz
                .arg("-ac") // Audio channels
                .arg("1") // Mono
                .arg(audio_path.to_str().unwrap()) // Output file
                .output()
                .expect("Failed to execute ffmpeg command");

            if !output.status.success() {
                panic!("ffmpeg command failed with output: {:?}", output);
            }

            audio_path
        }
        _ => path.to_path_buf(), // For audio files, use the original path
    };

    // Read the audio data into a buffer
    let reader = WavReader::open(audio_path).expect("Failed to open WAV file");
    let samples: Vec<f32> = reader
        .into_samples::<i16>()
        .map(|s| s.expect("Failed to read sample") as f32 / i16::MAX as f32)
        .collect();

    samples
}

use std::io::BufReader;
use std::process::Stdio;

pub fn py_whisper() {
    // Create input and output directories if they 't exist
    match fs::create_dir_all("whisper/input") {
        Ok(_) => println!("Input directory created successfully."),
        Err(err) => println!("Error creating input directory: {}", err),
    }
    match fs::create_dir_all("whisper/output/Audios") {
        Ok(_) => println!("Output directory created successfully."),
        Err(err) => println!("Error creating output directory: {}", err),
    }
    match fs::create_dir_all("whisper/output/Transcriptions") {
        Ok(_) => println!("Output directory created successfully."),
        Err(err) => println!("Error creating output directory: {}", err),
    }

    // Define commands for Unix-like and Windows systems
    let (command, script_file, arg) = if cfg!(target_os = "windows") {
        ("cmd", "run_whisper.bat", Some("/C"))
    } else {
        ("bash", "run_whisper.sh", None)
    };

    // Create the command
    let mut command = Command::new(command);
    if let Some(arg) = arg {
        command.arg(arg);
    }
    let mut child = command
        .arg(script_file)
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to execute command");

    // Create a BufReader for the stdout
    let reader = BufReader::new(child.stdout.take().expect("Failed to capture stdout"));

    // Read stdout line by line
    for line in reader.lines() {
        match line {
            Ok(line) => println!("{}", line),
            Err(err) => eprintln!("Failed to read line: {}", err),
        }
    }

    // Check if the child process has finished successfully
    let output = child.wait().expect("Failed to wait on child");
    if output.success() {
        println!("Script executed successfully");
    } else {
        eprintln!("Script failed");
    }
}
