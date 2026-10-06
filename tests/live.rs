//! Opt-in downloads of public examples. URLs can be overridden when posts expire.

use std::path::PathBuf;
use std::time::Duration;

use yt_dlp::client::deps::Libraries;
use yt_dlp::model::selector::{AudioCodecPreference, AudioQuality, VideoCodecPreference, VideoQuality};
use yt_dlp::{Downloader, VideoSelection};

async fn assert_live_download(platform: &str, default_url: &str, audio_only: bool) {
    let yt_dlp = PathBuf::from(std::env::var_os("YTDLP_TEST_BINARY").unwrap_or_else(|| "yt-dlp".into()));
    let ffmpeg = PathBuf::from(std::env::var_os("FFMPEG_TEST_BINARY").unwrap_or_else(|| "ffmpeg".into()));
    let url = std::env::var(format!("YTDLP_TEST_{platform}_URL")).unwrap_or_else(|_| default_url.to_string());
    let output_dir = tempfile::tempdir().unwrap();
    let downloader = Downloader::builder(Libraries::new(yt_dlp, ffmpeg.clone()), output_dir.path())
        .with_args(vec!["--ignore-config".into(), "--no-playlist".into()])
        .with_timeout(Duration::from_secs(120))
        .build()
        .await
        .unwrap();
    let video = downloader
        .fetch_video_infos(&url)
        .await
        .expect("Public media metadata should load");
    assert!(!video.formats.is_empty());

    let output = if audio_only {
        let format = video
            .select_audio_format(AudioQuality::Low, AudioCodecPreference::Any)
            .unwrap();
        output_dir
            .path()
            .join(format!("track.{}", format.download_info.ext.as_str()))
    } else {
        output_dir.path().join("video.mp4")
    };
    let path = downloader
        .download(&video, output)
        .video_quality(VideoQuality::Low)
        .video_codec(VideoCodecPreference::AVC1)
        .audio_quality(AudioQuality::Low)
        .execute()
        .await
        .expect("Public media should download");
    assert!(path.starts_with(output_dir.path()));
    assert!(std::fs::metadata(&path).unwrap().len() >= 1024);

    // A nonempty file alone could be an HTML error response from the CDN.
    let decoded = tokio::time::timeout(
        Duration::from_secs(30),
        tokio::process::Command::new(&ffmpeg)
            .args(["-v", "error", "-i"])
            .arg(&path)
            .args(["-t", "1", "-f", "null", "-"])
            .kill_on_drop(true)
            .output(),
    )
    .await
    .expect("Media validation should finish")
    .expect("FFmpeg should be available");
    assert!(decoded.status.success(), "Downloaded media should decode");

    if platform == "REDDIT" {
        let audio = tokio::process::Command::new(&ffmpeg)
            .args(["-v", "error", "-i"])
            .arg(&path)
            .args(["-map", "0:a:0", "-t", "1", "-f", "null", "-"])
            .output()
            .await
            .unwrap();
        assert!(
            audio.status.success(),
            "Reddit's separate audio stream should be preserved"
        );
    }
}

#[tokio::test]
#[ignore = "downloads a public YouTube video; requires yt-dlp and FFmpeg"]
async fn youtube() {
    assert_live_download("YOUTUBE", "https://www.youtube.com/watch?v=tCDvOQI3pco", false).await;
}

#[tokio::test]
#[ignore = "downloads a public TikTok video; requires yt-dlp and FFmpeg"]
async fn tiktok() {
    assert_live_download(
        "TIKTOK",
        "https://www.tiktok.com/@rickastleyofficial/video/7593022588272561430",
        false,
    )
    .await;
}

#[tokio::test]
#[ignore = "downloads a public Instagram reel; requires yt-dlp and FFmpeg"]
async fn instagram() {
    assert_live_download("INSTAGRAM", "https://www.instagram.com/reel/Chunk8-jurw/", false).await;
}

#[tokio::test]
#[ignore = "downloads a public SoundCloud track; requires yt-dlp and FFmpeg"]
async fn soundcloud() {
    assert_live_download(
        "SOUNDCLOUD",
        "https://soundcloud.com/ethmusic/lostin-powers-she-so-heavy",
        true,
    )
    .await;
}

#[tokio::test]
#[ignore = "downloads a public Facebook video; requires yt-dlp and FFmpeg"]
async fn facebook() {
    assert_live_download(
        "FACEBOOK",
        "https://www.facebook.com/cnn/videos/10155529876156509/",
        false,
    )
    .await;
}

#[tokio::test]
#[ignore = "downloads a public X video; requires yt-dlp and FFmpeg"]
async fn x() {
    assert_live_download("X", "https://x.com/captainamerica/status/719944021058060289", false).await;
}

#[tokio::test]
#[ignore = "downloads a public Reddit video with audio; requires yt-dlp and FFmpeg"]
async fn reddit() {
    assert_live_download(
        "REDDIT",
        "https://www.reddit.com/r/videos/comments/6rrwyj/that_small_heart_attack/",
        false,
    )
    .await;
}

#[tokio::test]
#[ignore = "downloads a silent Reddit video; requires yt-dlp and FFmpeg"]
async fn reddit_silent() {
    assert_live_download(
        "REDDIT_SILENT",
        "https://www.reddit.com/r/aww/comments/90bu6w/heat_index_was_110_degrees_so_we_offered_him_a/",
        false,
    )
    .await;
}
