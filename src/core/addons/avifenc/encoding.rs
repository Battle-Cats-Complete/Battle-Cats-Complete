use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::io::Write;
use std::path::PathBuf;
use crate::core::anim::export::encoding::{ExportConfig, EncoderMessage, EncoderStatus, prepare_image};
use super::download; 
use crate::core::addons::ffmpeg::download as ffmpeg_dl; 

pub fn encode(config: ExportConfig, rx: mpsc::Receiver<EncoderMessage>, status_tx: mpsc::Sender<EncoderStatus>, temp_path: &PathBuf) -> bool {
    let avif_path = match download::get_avif_path() { Some(p) => p, None => return false };
    let ffmpeg_path = match ffmpeg_dl::get_ffmpeg_path() { Some(p) => p, None => return false };

    let out_path_str = temp_path.to_string_lossy();
    
    // 1. Start Avifenc
    let mut avif_cmd = Command::new(avif_path)
        .args(&["--stdin", "--speed", "8", "-q", "60", "--qalpha", "60", "-o", &out_path_str])
        .stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().expect("Avifenc Fail");

    let avif_stdin = avif_cmd.stdin.take().expect("Stdin Fail");

    // 2. Start FFmpeg
    let mut ffmpeg_cmd = Command::new(ffmpeg_path)
        .args(&["-f", "rawvideo", "-pixel_format", "rgba", "-video_size", &format!("{}x{}", config.width, config.height), "-framerate", &config.fps.to_string(), "-i", "-", "-f", "yuv4mpegpipe", "-strict", "-1", "-pix_fmt", "yuva444p", "-"])
        .stdin(Stdio::piped()).stdout(Stdio::from(avif_stdin)).stderr(Stdio::null()).spawn().expect("FFmpeg Fail");

    let mut ff_stdin = ffmpeg_cmd.stdin.take().expect("FF Stdin Fail");
    let mut frames = 0;
    let mut success = false;

    // 3. Pumping Loop
    while let Ok(msg) = rx.recv() {
        match msg {
            EncoderMessage::Frame(raw_pixels, w, h, _) => {
                // ABORT DETECTION: If UI receiver is gone, kill everything and GTFO
                if status_tx.send(EncoderStatus::Progress(frames)).is_err() {
                    let _ = ffmpeg_cmd.kill();
                    let _ = avif_cmd.kill();
                    break; 
                }

                let img = prepare_image(raw_pixels, w, h);
                if ff_stdin.write_all(&img.into_vec()).is_err() {
                    break; // Child process likely died
                }
                frames += 1;
            },
            EncoderMessage::Finish => {
                success = true;
                break;
            }
        }
    }

    // 4. THE CLEANUP
    drop(ff_stdin); // Signal EOF to the pipe chain

    if !success {
        println!("DEBUG: Abort detected. Terminating processes...");
        let _ = ffmpeg_cmd.kill();
        let _ = avif_cmd.kill();
        // Wait for OS to release handles
        let _ = ffmpeg_cmd.wait();
        let _ = avif_cmd.wait();
        
        if temp_path.exists() { let _ = std::fs::remove_file(temp_path); }
    } else {
        // Normal finish: wait for them to close gracefully
        let _ = ffmpeg_cmd.wait();
        let s = avif_cmd.wait();
        success = s.map(|res| res.success()).unwrap_or(false);
    }

    success
}