use std::path::Path;

use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};
use yt_dlp::Downloader;
use yt_dlp::client::deps::Libraries;
use yt_dlp::model::Video;
use yt_dlp::model::selector::{VideoCodecPreference, VideoQuality};

use crate::common::fixtures;
use crate::helpers;

fn platform_video(name: &str, server: &str) -> Video {
    fixtures::load_fixture_with_url(&format!("platforms/{name}.json"), server)
}

#[tokio::test]
async fn facebook_progressive_download_respects_relative_output_and_referer() {
    let server = MockServer::start().await;
    let bytes = fixtures::load_media_bytes("small_fmp4.mp4");
    Mock::given(method("GET"))
        .and(path("/media"))
        .and(header("Referer", "https://www.facebook.com/"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(bytes.clone()))
        .mount(&server)
        .await;
    let tmp = fixtures::temp_test_dir();
    let downloader = helpers::build_e2e_downloader(&server.uri(), tmp.path()).await;
    let video = platform_video("facebook", &server.uri());
    let output = downloader
        .download(&video, "nested/facebook.mp4")
        .video_quality(VideoQuality::Low)
        .video_codec(VideoCodecPreference::AVC1)
        .execute()
        .await
        .unwrap();
    assert_eq!(output, tmp.path().join("nested/facebook.mp4"));
    assert_eq!(std::fs::read(output).unwrap(), bytes);
    let mut video = video;
    video.formats.retain(|format| format.format_id == "sd");
    let output = downloader.download_video(&video, "facebook-simple.mp4").await.unwrap();
    assert_eq!(output, tmp.path().join("facebook-simple.mp4"));
    assert_eq!(std::fs::read(output).unwrap(), bytes);
}

#[tokio::test]
async fn silent_reddit_video_downloads_without_an_audio_stream() {
    let server = MockServer::start().await;
    let bytes = fixtures::load_media_bytes("small_fmp4.mp4");
    Mock::given(method("GET"))
        .and(path("/media"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(bytes.clone()))
        .mount(&server)
        .await;
    let tmp = fixtures::temp_test_dir();
    let downloader = helpers::build_e2e_downloader(&server.uri(), tmp.path()).await;
    let mut video = platform_video("reddit", &server.uri());
    video.formats.retain(|format| format.format_id == "fallback");
    let output = downloader.download(&video, "silent.mp4").execute().await.unwrap();
    assert_eq!(output, tmp.path().join("silent.mp4"));
    assert_eq!(std::fs::read(output).unwrap(), bytes);
    let output = downloader.download_video(&video, "silent-simple.mp4").await.unwrap();
    assert_eq!(std::fs::read(output).unwrap(), bytes);
}

#[tokio::test]
async fn soundcloud_audio_only_builder_preserves_native_container() {
    let server = MockServer::start().await;
    let bytes = fixtures::load_media_bytes("small.m4a");
    Mock::given(method("GET"))
        .and(path("/media"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(bytes.clone()))
        .mount(&server)
        .await;
    let tmp = fixtures::temp_test_dir();
    let downloader = helpers::build_e2e_downloader(&server.uri(), tmp.path()).await;
    let mut video = platform_video("soundcloud", &server.uri());
    video.formats[0].download_info.url = Some(format!("{}/media", server.uri()));
    let output = downloader.download(&video, "track.m4a").execute().await.unwrap();
    assert_eq!(output, tmp.path().join("track.m4a"));
    assert_eq!(std::fs::read(output).unwrap(), bytes);
    let output = downloader.download_video(&video, "track-simple.m4a").await.unwrap();
    assert_eq!(std::fs::read(output).unwrap(), bytes);
}

#[cfg(unix)]
async fn recording_downloader(directory: &Path) -> Downloader {
    use std::os::unix::fs::PermissionsExt;

    fn quoted(path: &Path) -> String {
        format!("'{}'", path.to_str().unwrap().replace('\'', "'\\''"))
    }
    let binary = directory.join("yt-dlp-fixture.sh");
    let args_file = directory.join("args.txt");
    let source = fixtures::media_fixture("small_fmp4.mp4");
    // Record CLI routing and produce a local fixture without contacting a CDN.
    std::fs::write(&binary, format!(
        "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$@\" > {}\nwhile [ $# -gt 0 ]; do\n  if [ \"$1\" = '-o' ]; then cp {} \"$2\"; exit 0; fi\n  shift\ndone\nexit 1\n",
        quoted(&args_file), quoted(&source)
    )).unwrap();
    std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755)).unwrap();
    Downloader::builder(Libraries::new(binary, directory.join("ffmpeg")), directory)
        .build()
        .await
        .unwrap()
}

#[cfg(unix)]
#[tokio::test]
async fn tiktok_cookie_gated_download_respects_output_directory() {
    let tmp = fixtures::temp_test_dir();
    let downloader = recording_downloader(tmp.path()).await;
    let video = platform_video("tiktok", "https://example.com");
    let output = downloader
        .download(&video, "nested/tiktok.mp4")
        .execute()
        .await
        .unwrap();
    assert_eq!(output, tmp.path().join("nested/tiktok.mp4"));
    let args = std::fs::read_to_string(tmp.path().join("args.txt")).unwrap();
    assert!(args.contains("-f\ndownload\n"));
    assert!(output.is_file());
}

#[cfg(unix)]
#[tokio::test]
async fn x_hls_muxed_video_uses_the_extractor_context() {
    let tmp = fixtures::temp_test_dir();
    let downloader = recording_downloader(tmp.path()).await;
    let video = platform_video("x", "https://example.com");
    let output = downloader.download(&video, "x.mp4").execute().await.unwrap();
    assert_eq!(output, tmp.path().join("x.mp4"));
    let args = std::fs::read_to_string(tmp.path().join("args.txt")).unwrap();
    assert!(args.contains("-f\nhls-2176\n"));
    assert!(args.contains(video.webpage_url.as_deref().unwrap()));
}

#[cfg(unix)]
#[tokio::test]
async fn reddit_dash_streams_merge_using_configured_ffmpeg_and_mp4() {
    let tmp = fixtures::temp_test_dir();
    let downloader = recording_downloader(tmp.path()).await;
    let video = platform_video("reddit", "https://example.com");
    let output = downloader.download(&video, "reddit.mp4").execute().await.unwrap();
    assert_eq!(output, tmp.path().join("reddit.mp4"));
    let args = std::fs::read_to_string(tmp.path().join("args.txt")).unwrap();
    assert!(args.contains("-f\ndash-VIDEO-1+dash-AUDIO-1\n"));
    assert!(args.contains(&format!("--ffmpeg-location\n{}\n", tmp.path().join("ffmpeg").display())));
    assert!(args.contains("--merge-output-format\nmp4\n"));
    let output = downloader.download_video(&video, "reddit-simple.mp4").await.unwrap();
    assert_eq!(output, tmp.path().join("reddit-simple.mp4"));
    let args = std::fs::read_to_string(tmp.path().join("args.txt")).unwrap();
    assert!(args.contains("-f\ndash-VIDEO-1+dash-AUDIO-1\n"));
}
