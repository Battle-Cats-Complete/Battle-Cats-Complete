use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::path::PathBuf;
use std::thread;
use std::io::{Write, Read};
use std::fs;
use crate::core::anim::export::encoding::{ExportConfig, EncoderMessage, EncoderStatus, prepare_image};
use super::download;

pub fn encode(config: ExportConfig, rx: mpsc::Receiver<EncoderMessage>, status_tx: mpsc::Sender<EncoderStatus>, temp_path: &PathBuf) -> bool {
    let ffmpeg_path = match download::get_ffmpeg_path() { Some(p) => p, None => return false };

    let mut child = Command::new(ffmpeg_path)
        .args(&["-f", "rawvideo", "-pixel_format", "rgba", "-video_size", &format!("{}x{}", config.width, config.height), "-framerate", &config.fps.to_string(), "-i", "-", "-vf", "split[s0][s1];[s0]palettegen[p];[s1][p]paletteuse", "-f", "gif", "-"])
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().expect("FFmpeg Fail");

    let mut stdin = child.stdin.take().expect("Stdin Fail");
    let mut stdout = child.stdout.take().expect("Stdout Fail");
    let tx_progress = status_tx.clone();
    
    // Thread 1: Pump frames in
    let input_handle = thread::spawn(move || {
        let mut frames = 0;
        let mut clean = false;
        while let Ok(msg) = rx.recv() {
            match msg {
                EncoderMessage::Frame(raw_pixels, w, h, _) => {
                    if tx_progress.send(EncoderStatus::Progress(frames)).is_err() { break; } // ABORT
                    let img = prepare_image(raw_pixels, w, h);
                    if stdin.write_all(&img.into_vec()).is_err() { break; }
                    frames += 1;
                },
                EncoderMessage::Finish => { clean = true; break; }
            }
        }
        drop(stdin);
        clean
    });

    // Main Thread: Pull bytes out into RAM
    let mut gif_buffer = Vec::new();
    let read_result = stdout.read_to_end(&mut gif_buffer);
    
    let input_success = input_handle.join().unwrap_or(false);
    let process_success = child.wait().map(|s| s.success()).unwrap_or(false);

    if input_success && process_success && read_result.is_ok() {
        let _ = fs::write(temp_path, gif_buffer);
        true
    } else {
        // FORCE KILL if anything failed
        let _ = child.kill();
        let _ = child.wait();
        false
    }
}