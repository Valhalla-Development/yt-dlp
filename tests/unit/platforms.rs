use yt_dlp::VideoSelection;
use yt_dlp::model::Video;
use yt_dlp::model::format::FormatType;
use yt_dlp::model::selector::{AudioCodecPreference, AudioQuality, VideoCodecPreference, VideoQuality};

use crate::common::fixtures;

// Reduced extractor outputs preserve the missing fields and format layouts that
// previously broke the wrapper. Stream URLs and cookie values are placeholders.
fn platform_video(name: &str) -> Video {
    let json = fixtures::load_json_string(&format!("platforms/{name}.json"));
    serde_json::from_str(&json).expect("Platform metadata should deserialize")
}

#[test]
fn youtube_selects_separate_video_and_audio_streams() {
    let video = platform_video("youtube");
    let selected = video
        .select_video_format(VideoQuality::Best, VideoCodecPreference::AVC1)
        .unwrap();
    assert_eq!(selected.format_id, "137");
    assert_eq!(selected.format_type(), FormatType::Video);
    assert_eq!(
        video
            .select_audio_format(AudioQuality::Best, AudioCodecPreference::AAC)
            .unwrap()
            .format_id,
        "140"
    );
}

#[test]
fn tiktok_accepts_missing_metadata_and_preserves_cdn_authentication() {
    let video = platform_video("tiktok");
    assert_eq!(video.age_limit, 0);
    assert_eq!(video.live_status, "");
    assert!(!video.playable_in_embed);
    let format = video
        .select_video_format(VideoQuality::Best, VideoCodecPreference::Any)
        .unwrap();
    assert_eq!(format.format_type(), FormatType::AudioVideo);
    assert_eq!(format.download_info.http_headers.referer, "https://www.tiktok.com/");
    assert_eq!(
        format.download_info.cookies.as_deref(),
        Some("fixture_session=placeholder")
    );
}

#[test]
fn instagram_accepts_optional_thumbnails_and_selects_progressive_mp4() {
    let video = platform_video("instagram");
    assert_eq!(video.thumbnails[0].preference, 0);
    let format = video
        .select_video_format(VideoQuality::Best, VideoCodecPreference::AVC1)
        .unwrap();
    assert_eq!(format.format_id, "3");
    assert_eq!(format.format_type(), FormatType::AudioVideo);
}

#[test]
fn soundcloud_preserves_fractional_duration_and_selects_audio() {
    let video = platform_video("soundcloud");
    assert_eq!(video.duration, Some(143.206));
    assert!(
        video
            .select_video_format(VideoQuality::Best, VideoCodecPreference::Any)
            .is_none()
    );
    let format = video
        .select_audio_format(AudioQuality::Best, AudioCodecPreference::Any)
        .unwrap();
    assert_eq!(format.format_type(), FormatType::Audio);
    assert_eq!(format.download_info.ext.as_str(), "m4a");
}

#[test]
fn facebook_selects_unlabeled_progressive_video() {
    let video = platform_video("facebook");
    let format = video
        .select_video_format(VideoQuality::Low, VideoCodecPreference::AVC1)
        .unwrap();
    assert_eq!(format.format_id, "sd");
    assert_eq!(format.format_type(), FormatType::AudioVideo);
}

#[test]
fn x_keeps_muxed_hls_selectable() {
    let video = platform_video("x");
    let format = video
        .select_video_format(VideoQuality::Best, VideoCodecPreference::AVC1)
        .unwrap();
    assert_eq!(format.format_id, "hls-2176");
    assert_eq!(format.format_type(), FormatType::AudioVideo);
}

#[test]
fn reddit_selects_dash_video_and_audio_despite_manifest_urls() {
    let video = platform_video("reddit");
    assert_eq!(video.upload_date, Some(1501941939));
    let format = video
        .select_video_format(VideoQuality::Best, VideoCodecPreference::AVC1)
        .unwrap();
    assert_eq!(format.format_id, "dash-VIDEO-1");
    assert_eq!(format.format_type(), FormatType::Video);
    let audio = video
        .select_audio_format(AudioQuality::Best, AudioCodecPreference::Any)
        .unwrap();
    assert_eq!(audio.format_id, "dash-AUDIO-1");
    assert_eq!(audio.format_type(), FormatType::Audio);
}

#[test]
fn timestamps_accept_numeric_seconds_without_weakening_validation() {
    let base = serde_json::to_value(platform_video("reddit")).unwrap();
    for field in ["timestamp", "release_timestamp"] {
        for (value, expected) in [
            (serde_json::json!(1501941939), Some(1501941939)),
            (serde_json::json!(1501941939.75), Some(1501941939)),
            (serde_json::json!(null), None),
            (serde_json::json!(i64::MAX), Some(i64::MAX)),
        ] {
            let mut json = base.clone();
            json[field] = value;
            let video: Video = serde_json::from_value(json).unwrap();
            assert_eq!(
                if field == "timestamp" {
                    video.upload_date
                } else {
                    video.release_timestamp
                },
                expected
            );
        }
        for value in [
            serde_json::json!("1501941939"),
            serde_json::json!(1e30),
            serde_json::json!(-1e30),
        ] {
            let mut json = base.clone();
            json[field] = value;
            assert!(serde_json::from_value::<Video>(json).is_err());
        }
        let mut json = base.clone();
        json.as_object_mut().unwrap().remove(field);
        assert!(serde_json::from_value::<Video>(json).is_ok());
    }
}
