use crate::tools::Tool;
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::process::Command;

pub struct VideoConvertTool;

impl Tool for VideoConvertTool {
    fn name(&self) -> String {
        "video_convert".to_string()
    }

    fn description(&self) -> String {
        "Convert video formats or extract audio using FFmpeg.".to_string()
    }

    fn schema(&self) -> Value {
        json!({
            "name": "video_convert",
            "description": "Convert video formats or extract audio using FFmpeg.",
            "parameters": {
                "type": "object",
                "properties": {
                    "input_path": { "type": "string" },
                    "output_path": { "type": "string" },
                    "crf": { "type": "integer", "description": "Quality (0-51, default 23). Lower is better." },
                    "preset": { "type": "string", "description": "Encoding speed (ultrafast to veryslow)" }
                },
                "required": ["input_path", "output_path"]
            }
        })
    }

    fn execute(&self, args: Value) -> Result<String> {
        let input = args["input_path"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing input_path"))?;
        let output = args["output_path"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing output_path"))?;
        let crf = args["crf"].as_i64().unwrap_or(23).to_string();
        let preset = args["preset"].as_str().unwrap_or("medium");

        let status = Command::new("ffmpeg")
            .arg("-i")
            .arg(input)
            .arg("-c:v")
            .arg("libx264")
            .arg("-crf")
            .arg(crf)
            .arg("-preset")
            .arg(preset)
            .arg("-y") // Overwrite
            .arg(output)
            .status()?;

        if status.success() {
            Ok(format!("Successfully converted {} to {}", input, output))
        } else {
            Err(anyhow!("FFmpeg conversion failed"))
        }
    }
}

pub struct VideoCutTool;

impl Tool for VideoCutTool {
    fn name(&self) -> String {
        "video_cut".to_string()
    }

    fn description(&self) -> String {
        "Trim video start/end.".to_string()
    }

    fn schema(&self) -> Value {
        json!({
            "name": "video_cut",
            "description": "Trim video start/end.",
            "parameters": {
                "type": "object",
                "properties": {
                    "input_path": { "type": "string" },
                    "start_time": { "type": "string", "description": "Start timestamp (e.g. 00:00:03)" },
                    "duration": { "type": "string", "description": "Duration to keep (e.g. 10)" },
                    "output_path": { "type": "string" }
                },
                "required": ["input_path", "start_time", "output_path"]
            }
        })
    }

    fn execute(&self, args: Value) -> Result<String> {
        let input = args["input_path"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing input_path"))?;
        let output = args["output_path"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing output_path"))?;
        let start = args["start_time"].as_str().unwrap_or("00:00:00");

        let mut cmd = Command::new("ffmpeg");
        cmd.arg("-ss").arg(start).arg("-i").arg(input);

        if let Some(dur) = args["duration"].as_str() {
            cmd.arg("-t").arg(dur);
        }

        // Use copy codec for speed if possible, or re-encode if exact cutting needed?
        // Let's re-encode to be safe on keyframes
        let status = cmd
            .arg("-c")
            .arg("copy") // Try stream copy first for speed
            .arg("-y")
            .arg(output)
            .status()?;

        if status.success() {
            Ok(format!("Successfully cut video to {}", output))
        } else {
            Err(anyhow!("FFmpeg cut failed"))
        }
    }
}

pub struct VideoGifTool;

impl Tool for VideoGifTool {
    fn name(&self) -> String {
        "video_to_gif".to_string()
    }

    fn description(&self) -> String {
        "Convert video to high-quality GIF.".to_string()
    }

    fn schema(&self) -> Value {
        json!({
            "name": "video_to_gif",
            "description": "Convert video to high-quality GIF using palettegen.",
            "parameters": {
                "type": "object",
                "properties": {
                    "input_path": { "type": "string" },
                    "output_path": { "type": "string" },
                    "width": { "type": "integer", "description": "Width in pixels (default 640)" },
                    "fps": { "type": "integer", "description": "Frames per second (default 15)" }
                },
                "required": ["input_path", "output_path"]
            }
        })
    }

    fn execute(&self, args: Value) -> Result<String> {
        let input = args["input_path"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing input_path"))?;
        let output = args["output_path"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing output_path"))?;
        let width = args["width"].as_i64().unwrap_or(640);
        let fps = args["fps"].as_i64().unwrap_or(15);

        // Complex filter for palette generation
        let filter = format!(
            "fps={},scale={}:-1:flags=lanczos,split[s0][s1];[s0]palettegen[p];[s1][p]paletteuse",
            fps, width
        );

        let status = Command::new("ffmpeg")
            .arg("-i")
            .arg(input)
            .arg("-vf")
            .arg(filter)
            .arg("-y")
            .arg(output)
            .status()?;

        if status.success() {
            Ok(format!("Successfully created GIF at {}", output))
        } else {
            Err(anyhow!("FFmpeg GIF conversion failed"))
        }
    }
}

pub struct VideoProbeTool;

impl Tool for VideoProbeTool {
    fn name(&self) -> String {
        "video_info".to_string()
    }

    fn description(&self) -> String {
        "Get video metadata using ffprobe.".to_string()
    }

    fn schema(&self) -> Value {
        json!({
            "name": "video_info",
            "description": "Get video metadata.",
            "parameters": {
                "type": "object",
                "properties": {
                    "path": { "type": "string" }
                },
                "required": ["path"]
            }
        })
    }

    fn execute(&self, args: Value) -> Result<String> {
        let path = args["path"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing path"))?;

        let output = Command::new("ffprobe")
            .arg("-v")
            .arg("error")
            .arg("-show_entries")
            .arg("stream=width,height,duration,r_frame_rate,codec_name")
            .arg("-of")
            .arg("json")
            .arg(path)
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        if output.status.success() {
            Ok(stdout.to_string())
        } else {
            Err(anyhow!(
                "FFprobe failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    }
}
