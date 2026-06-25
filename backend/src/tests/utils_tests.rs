use crate::utils::{is_private_ip, get_safe_filename, TempFileGuard};
use std::net::IpAddr;
use std::str::FromStr;
use std::fs;

#[test]
fn test_is_private_ip() {
    // IPv4 Loopback & Private ranges
    assert!(is_private_ip(IpAddr::from_str("127.0.0.1").unwrap()));
    assert!(is_private_ip(IpAddr::from_str("10.0.0.1").unwrap()));
    assert!(is_private_ip(IpAddr::from_str("172.16.31.254").unwrap()));
    assert!(is_private_ip(IpAddr::from_str("192.168.1.1").unwrap()));

    // IPv4 CGNAT
    assert!(is_private_ip(IpAddr::from_str("100.64.0.5").unwrap()));
    assert!(is_private_ip(IpAddr::from_str("100.127.255.255").unwrap()));

    // IPv4 Documentation & Benchmarking & Link-local
    assert!(is_private_ip(IpAddr::from_str("192.0.2.1").unwrap()));
    assert!(is_private_ip(IpAddr::from_str("198.51.100.2").unwrap()));
    assert!(is_private_ip(IpAddr::from_str("203.0.113.3").unwrap()));
    assert!(is_private_ip(IpAddr::from_str("198.18.0.4").unwrap()));
    assert!(is_private_ip(IpAddr::from_str("169.254.0.1").unwrap()));
    assert!(is_private_ip(IpAddr::from_str("0.0.0.0").unwrap()));
    assert!(is_private_ip(IpAddr::from_str("240.0.0.1").unwrap()));

    // IPv4 Public
    assert!(!is_private_ip(IpAddr::from_str("8.8.8.8").unwrap()));
    assert!(!is_private_ip(IpAddr::from_str("1.1.1.1").unwrap()));
    assert!(!is_private_ip(IpAddr::from_str("104.26.10.19").unwrap()));

    // IPv6 Loopback & Unspecified & Link-local & Unique local
    assert!(is_private_ip(IpAddr::from_str("::1").unwrap()));
    assert!(is_private_ip(IpAddr::from_str("::").unwrap()));
    assert!(is_private_ip(IpAddr::from_str("fe80::1").unwrap()));
    assert!(is_private_ip(IpAddr::from_str("fc00::1").unwrap()));
    assert!(is_private_ip(IpAddr::from_str("fdff::1").unwrap()));

    // IPv6 IPv4-mapped (::ffff:0:0/96)
    assert!(is_private_ip(IpAddr::from_str("::ffff:192.168.1.1").unwrap()));
    assert!(!is_private_ip(IpAddr::from_str("::ffff:8.8.8.8").unwrap()));

    // IPv6 IPv4-compatible (::/96)
    assert!(is_private_ip(IpAddr::from_str("::192.168.1.1").unwrap()));
    assert!(is_private_ip(IpAddr::from_str("::127.0.0.1").unwrap()));
    assert!(!is_private_ip(IpAddr::from_str("::8.8.8.8").unwrap()));

    // NAT64 prefix (64:ff9b::/96)
    assert!(is_private_ip(IpAddr::from_str("64:ff9b::192.168.1.1").unwrap()));
    assert!(is_private_ip(IpAddr::from_str("64:ff9b::127.0.0.1").unwrap()));
    assert!(!is_private_ip(IpAddr::from_str("64:ff9b::8.8.8.8").unwrap()));

    // IPv6 Documentation (2001:db8::/32)
    assert!(is_private_ip(IpAddr::from_str("2001:db8::1").unwrap()));

    // IPv6 Discard Prefix (100::/64)
    assert!(is_private_ip(IpAddr::from_str("100::1").unwrap()));

    // IPv6 Public
    assert!(!is_private_ip(IpAddr::from_str("2606:4700:4700::1111").unwrap()));
    assert!(!is_private_ip(IpAddr::from_str("2001:4860:4860::8888").unwrap()));
}

#[test]
fn test_is_valid_video_signature() {
    use crate::utils::is_valid_video_signature;

    // MKV / WebM
    assert!(is_valid_video_signature(&[0x1A, 0x45, 0xDF, 0xA3]));
    assert!(is_valid_video_signature(&[0x1A, 0x45, 0xDF, 0xA3, 0x01, 0x02]));

    // MP4
    let mut mp4_sig = vec![0x00, 0x00, 0x00, 0x18];
    mp4_sig.extend_from_slice(b"ftypmp42");
    assert!(is_valid_video_signature(&mp4_sig));

    // AVI
    let avi_sig = b"RIFF\x00\x00\x00\x00AVI ".to_vec();
    assert!(is_valid_video_signature(&avi_sig));

    // MPEG
    assert!(is_valid_video_signature(&[0x00, 0x00, 0x01, 0xBA]));
    assert!(is_valid_video_signature(&[0x00, 0x00, 0x01, 0xB3]));

    // Invalid signatures
    assert!(!is_valid_video_signature(&[0x00, 0x00, 0x00, 0x00]));
    assert!(!is_valid_video_signature(b"MZ\x90\x00")); // EXE
    assert!(!is_valid_video_signature(b"PK\x03\x04")); // ZIP
    assert!(!is_valid_video_signature(&[]));
}

#[test]
fn test_get_safe_filename() {
    // Simple file path segment
    let url = reqwest::Url::parse("https://example.com/some/path/movie.mp4").unwrap();
    assert_eq!(get_safe_filename(&url, None), "movie.mp4");

    // URL encoded characters
    let url = reqwest::Url::parse("https://example.com/some/path/my%20cool%20video.mp4").unwrap();
    assert_eq!(get_safe_filename(&url, None), "mycoolvideo.mp4"); // alphanumeric + . + - + _

    // Path Traversal check (URL path segment resolved by parser, default extension added)
    let url = reqwest::Url::parse("https://example.com/some/path/../../etc/passwd").unwrap();
    assert_eq!(get_safe_filename(&url, None), "passwd.mp4");

    // URL-encoded path traversal check (decoded and sanitized)
    let url = reqwest::Url::parse("https://example.com/some/path/%2e%2e%2f%2e%2e%2fetc%2fpasswd").unwrap();
    assert_eq!(get_safe_filename(&url, None), "....etcpasswd");

    // Fallbacks
    let url = reqwest::Url::parse("https://example.com/").unwrap();
    assert_eq!(get_safe_filename(&url, None), "video.mp4");

    let url = reqwest::Url::parse("https://example.com/..").unwrap();
    assert_eq!(get_safe_filename(&url, None), "video.mp4");

    // Extension mappings
    let url = reqwest::Url::parse("https://example.com/my-video").unwrap();
    assert_eq!(get_safe_filename(&url, Some("video/mp4")), "my-video.mp4");
    assert_eq!(get_safe_filename(&url, Some("video/webm")), "my-video.webm");
    assert_eq!(get_safe_filename(&url, Some("video/mkv")), "my-video.mkv");
    assert_eq!(get_safe_filename(&url, Some("video/x-matroska")), "my-video.mkv");
    assert_eq!(get_safe_filename(&url, Some("video/avi")), "my-video.avi");
    assert_eq!(get_safe_filename(&url, Some("video/quicktime")), "my-video.mov");
    assert_eq!(get_safe_filename(&url, Some("application/octet-stream")), "my-video.mp4"); // fallback
    assert_eq!(get_safe_filename(&url, None), "my-video.mp4"); // fallback
}

#[test]
fn test_temp_file_guard() {
    let temp_dir = std::env::temp_dir();
    let temp_file_path = temp_dir.join("test_temp_file_guard_antigravity.tmp");

    // Create the file first
    fs::write(&temp_file_path, b"test data").unwrap();
    assert!(temp_file_path.exists());

    // Test guard dropping and deleting file
    {
        let _guard = TempFileGuard::new(temp_file_path.clone());
    }
    assert!(!temp_file_path.exists());

    // Test guard deactivation (no deletion)
    fs::write(&temp_file_path, b"test data").unwrap();
    assert!(temp_file_path.exists());
    {
        let mut guard = TempFileGuard::new(temp_file_path.clone());
        guard.deactivate();
    }
    assert!(temp_file_path.exists());

    // Clean up
    let _ = fs::remove_file(&temp_file_path);
}
