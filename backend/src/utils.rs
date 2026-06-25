pub fn is_private_ip(ip: std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(ipv4) => {
            let octets = ipv4.octets();
            ipv4.is_loopback()
                || ipv4.is_private()
                || ipv4.is_link_local()
                || ipv4.is_multicast()
                || ipv4.is_unspecified()
                || octets[0] == 0 // Current network
                || octets[0] >= 240 // Reserved / Class E
                // CGNAT / Shared Address Space (100.64.0.0/10)
                || (octets[0] == 100 && (octets[1] & 0xc0) == 64)
                // Documentation blocks (192.0.2.0/24, 198.51.100.0/24, 203.0.113.0/24)
                || (octets[0] == 192 && octets[1] == 0 && octets[2] == 2)
                || (octets[0] == 198 && octets[1] == 51 && octets[2] == 100)
                || (octets[0] == 203 && octets[1] == 0 && octets[2] == 113)
                // Benchmarking (198.18.0.0/15)
                || (octets[0] == 198 && (octets[1] & 0xfe) == 18)
        }
        std::net::IpAddr::V6(ipv6) => {
            let octets = ipv6.octets();

            // 1. Chuẩn hóa IPv4-compatible (::x.x.x.x) và IPv4-mapped (::ffff:x.x.x.x)
            if let Some(ipv4) = ipv6.to_ipv4() {
                return is_private_ip(std::net::IpAddr::V4(ipv4));
            }

            // 2. Chặn tiền tố NAT64 tiêu chuẩn (64:ff9b::/96)
            if octets[..12] == [0, 0x64, 0xff, 0x9b, 0, 0, 0, 0, 0, 0, 0, 0] {
                let mapped_ipv4 = std::net::Ipv4Addr::new(octets[12], octets[13], octets[14], octets[15]);
                return is_private_ip(std::net::IpAddr::V4(mapped_ipv4));
            }

            // 3. Các dải IPv6 Private/Loopback tiêu chuẩn
            ipv6.is_loopback()
                || ipv6.is_unspecified()
                || ipv6.is_multicast()
                // Check for link-local (fe80::/10)
                || (octets[0] == 0xfe && (octets[1] & 0xc0) == 0x80)
                // Check for unique local (fc00::/7)
                || ((octets[0] & 0xfe) == 0xfc)
                // Documentation (2001:db8::/32)
                || (octets[0] == 0x20 && octets[1] == 0x01 && octets[2] == 0x0d && octets[3] == 0xb8)
                // Discard Prefix (100::/64)
                || octets[..8] == [0x01, 0x00, 0, 0, 0, 0, 0, 0]
        }
    }
}

pub fn is_valid_video_signature(data: &[u8]) -> bool {
    if data.len() < 4 {
        return false;
    }

    // 1. MKV/WebM (EBML: 1A 45 DF A3)
    if data[..4] == [0x1A, 0x45, 0xDF, 0xA3] {
        return true;
    }

    // 2. MP4/MOV (ftyp at offset 4, hoặc 3GP)
    // Cần tối thiểu 8 byte để đọc chữ ký ftyp ở byte 4-7
    if data.len() >= 8 && &data[4..8] == b"ftyp" {
        return true;
    }

    // 3. AVI (RIFF ở 0-3 và AVI ở 8-11)
    if data.len() >= 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"AVI " {
        return true;
    }

    // 4. MPEG (00 00 01 BA hoặc 00 00 01 B3)
    if data[..4] == [0x00, 0x00, 0x01, 0xBA] || data[..4] == [0x00, 0x00, 0x01, 0xB3] {
        return true;
    }

    false
}

pub struct TempFileGuard {
    path: std::path::PathBuf,
    active: bool,
}

impl TempFileGuard {
    pub fn new(path: std::path::PathBuf) -> Self {
        Self { path, active: true }
    }

    #[allow(dead_code)]
    pub fn deactivate(&mut self) {
        self.active = false;
    }
}

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        if self.active && self.path.exists() {
            if let Err(e) = std::fs::remove_file(&self.path) {
                tracing::warn!("TempFileGuard: Lỗi khi tự động xóa file tạm {:?}: {}", self.path, e);
            } else {
                tracing::info!("TempFileGuard: Đã tự động dọn dẹp file tạm {:?}", self.path);
            }
        }
    }
}

pub fn get_safe_filename(url: &reqwest::Url, content_type: Option<&str>) -> String {
    let name = url.path_segments()
        .and_then(|segments| segments.last())
        .unwrap_or("video.mp4")
        .to_string();
    
    // Giải mã ký tự URL-encode
    let mut decoded_name = urlencoding::decode(&name)
        .map(|cow| cow.into_owned())
        .unwrap_or(name);
    
    // Loại bỏ các ký tự không an toàn trong tên file
    decoded_name = decoded_name.replace(|c: char| !c.is_alphanumeric() && c != '.' && c != '-' && c != '_', "");
    
    if decoded_name.is_empty() || decoded_name == "." || decoded_name == ".." {
        decoded_name = "video.mp4".to_string();
    }
    
    // Nếu chưa có đuôi file, cố gắng map từ Content-Type
    let has_ext = std::path::Path::new(&decoded_name).extension().is_some();
    if !has_ext {
        if let Some(ct) = content_type {
            let ext = match ct {
                "video/mp4" => "mp4",
                "video/webm" => "webm",
                "video/x-matroska" | "video/mkv" => "mkv",
                "video/avi" | "video/x-msvideo" => "avi",
                "video/quicktime" => "mov",
                _ => "mp4",
            };
            decoded_name = format!("{}.{}", decoded_name, ext);
        } else {
            decoded_name = format!("{}.mp4", decoded_name);
        }
    }
    
    decoded_name
}

