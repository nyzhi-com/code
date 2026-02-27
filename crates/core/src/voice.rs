use anyhow::{Context, Result};
use std::path::Path;

const WHISPER_API_URL: &str = "https://api.openai.com/v1/audio/transcriptions";
const DEFAULT_MODEL: &str = "whisper-1";

pub struct VoiceConfig {
    pub api_key: String,
    pub model: String,
    pub language: Option<String>,
}

impl VoiceConfig {
    pub fn from_env() -> Option<Self> {
        let key = std::env::var("OPENAI_API_KEY").ok()?;
        Some(Self {
            api_key: key,
            model: DEFAULT_MODEL.to_string(),
            language: None,
        })
    }

    pub fn from_key(api_key: String) -> Self {
        Self {
            api_key,
            model: DEFAULT_MODEL.to_string(),
            language: None,
        }
    }
}

/// Transcribe an audio file using OpenAI Whisper API.
pub async fn transcribe(config: &VoiceConfig, audio_path: &Path) -> Result<String> {
    let file_bytes = tokio::fs::read(audio_path)
        .await
        .context("Failed to read audio file")?;

    let file_name = audio_path
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| "audio.wav".to_string());

    let mime = if file_name.ends_with(".mp3") {
        "audio/mpeg"
    } else if file_name.ends_with(".m4a") {
        "audio/mp4"
    } else if file_name.ends_with(".ogg") {
        "audio/ogg"
    } else if file_name.ends_with(".flac") {
        "audio/flac"
    } else {
        "audio/wav"
    };

    let file_part = reqwest::multipart::Part::bytes(file_bytes)
        .file_name(file_name)
        .mime_str(mime)?;

    let mut form = reqwest::multipart::Form::new()
        .part("file", file_part)
        .text("model", config.model.clone())
        .text("response_format", "text");

    if let Some(ref lang) = config.language {
        form = form.text("language", lang.clone());
    }

    let client = reqwest::Client::new();
    let resp = client
        .post(WHISPER_API_URL)
        .header("Authorization", format!("Bearer {}", config.api_key))
        .multipart(form)
        .send()
        .await
        .context("Whisper API request failed")?;

    let status = resp.status();
    let body = resp.text().await.context("Failed to read Whisper response")?;

    if !status.is_success() {
        anyhow::bail!("Whisper API error ({}): {}", status, body);
    }

    Ok(body.trim().to_string())
}

/// Record audio from the default microphone using system tools.
/// Returns the path to the recorded WAV file.
pub async fn record_audio(output_dir: &Path, duration_secs: u32) -> Result<std::path::PathBuf> {
    let output_path = output_dir.join("nyzhi_voice_input.wav");
    let _ = std::fs::create_dir_all(output_dir);

    #[cfg(target_os = "macos")]
    {
        let status = tokio::process::Command::new("sox")
            .args([
                "-d",
                "-r", "16000",
                "-c", "1",
                "-b", "16",
                output_path.to_str().unwrap_or("nyzhi_voice_input.wav"),
                "trim", "0",
                &duration_secs.to_string(),
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .await;

        match status {
            Ok(s) if s.success() => return Ok(output_path),
            _ => {
                let status2 = tokio::process::Command::new("rec")
                    .args([
                        "-r", "16000",
                        "-c", "1",
                        "-b", "16",
                        output_path.to_str().unwrap_or("nyzhi_voice_input.wav"),
                        "trim", "0",
                        &duration_secs.to_string(),
                    ])
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status()
                    .await;

                match status2 {
                    Ok(s) if s.success() => return Ok(output_path),
                    _ => anyhow::bail!(
                        "No audio recording tool found. Install sox: brew install sox"
                    ),
                }
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        let status = tokio::process::Command::new("arecord")
            .args([
                "-d", &duration_secs.to_string(),
                "-f", "S16_LE",
                "-r", "16000",
                "-c", "1",
                output_path.to_str().unwrap_or("nyzhi_voice_input.wav"),
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .await;

        match status {
            Ok(s) if s.success() => return Ok(output_path),
            _ => anyhow::bail!(
                "No audio recording tool found. Install alsa-utils: sudo apt install alsa-utils"
            ),
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = duration_secs;
        anyhow::bail!("Voice recording not supported on this platform")
    }
}

pub fn is_available() -> bool {
    VoiceConfig::from_env().is_some()
}

pub fn status_message() -> String {
    if is_available() {
        "Voice input available. Use Ctrl+V to record (5s), or /voice <seconds>.".to_string()
    } else {
        "Voice input requires OPENAI_API_KEY.\n\
         Set the env var or configure [voice] in config.toml.\n\
         Once configured, use Ctrl+V to toggle recording."
            .to_string()
    }
}
