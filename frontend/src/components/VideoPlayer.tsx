import React, { useState, useEffect, useRef, useCallback } from 'react';
import { Loader2, PlayCircle, ShieldAlert } from 'lucide-react';
import { Video } from '../types';
import { videoApi } from '../api/videoApi';
import { cleanVideoName } from '../utils/formatters';

interface VideoPlayerProps {
  video: Video | null;
}

export const VideoPlayer: React.FC<VideoPlayerProps> = ({ video }) => {
  const [streamUrl, setStreamUrl] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [aspectRatio, setAspectRatio] = useState<string>('16/9');
  const videoRef = useRef<HTMLVideoElement>(null);

  const fetchStreamUrl = useCallback(async () => {
    if (!video) return;
    setLoading(true);
    setError(null);
    setStreamUrl(null);
    setAspectRatio('16/9');

    try {
      const response = await videoApi.getStreamUrl(video.key);
      setStreamUrl(response.stream_url);
    } catch (err: any) {
      console.error("Lỗi lấy Stream URL từ API:", err);
      setError(err.response?.data?.error || 'Không thể lấy đường dẫn stream từ máy chủ.');
    } finally {
      setLoading(false);
    }
  }, [video]);

  useEffect(() => {
    if (!video) {
      setStreamUrl(null);
      setError(null);
      setAspectRatio('16/9');
      return;
    }

    fetchStreamUrl();
  }, [video, fetchStreamUrl]);

  // Reload video element when stream URL changes
  useEffect(() => {
    const currentVideo = videoRef.current;

    if (currentVideo && streamUrl) {
      currentVideo.load();
      const playPromise = currentVideo.play();
      if (playPromise !== undefined) {
        playPromise.catch((err) => {
          console.warn("Autoplay was prevented on Safari/iOS:", err);
        });
      }
    }

    return () => {
      if (currentVideo) {
        currentVideo.pause();
        currentVideo.src = "";
        try {
          currentVideo.load();
        } catch (_) {}
      }
    };
  }, [streamUrl]);

  const handleVideoError = (e: React.SyntheticEvent<HTMLVideoElement, Event>) => {
    const videoEl = e.currentTarget;
    const mediaError = videoEl.error;
    
    console.error("====== Safe Stream - LỖI PHÁT VIDEO ======");
    console.error("Video Key:", video?.key);
    console.error("Stream URL:", streamUrl);
    console.error("Media Error object:", mediaError);
    
    let errMsg = "Không thể phát video này.";
    if (mediaError) {
      console.error(`MediaError Code: ${mediaError.code}, Message: ${mediaError.message}`);
      switch (mediaError.code) {
        case 1: // MEDIA_ERR_ABORTED
          errMsg = "Quá trình tải video bị hủy bỏ.";
          break;
        case 2: // MEDIA_ERR_NETWORK
          errMsg = "Lỗi kết nối mạng khi tải video. Rất có thể do Cloudflare R2 chặn CORS hoặc kết nối mạng không ổn định.";
          break;
        case 3: // MEDIA_ERR_DECODE
          errMsg = "Lỗi giải mã video. Định dạng video không được hỗ trợ bởi trình duyệt này hoặc file bị hỏng.";
          break;
        case 4: // MEDIA_ERR_SRC_NOT_SUPPORTED
          errMsg = "Không thể tải video. Có thể do tên miền R2 chưa được thêm vào Content-Security-Policy (CSP) của Backend, hoặc liên kết hết hạn, hoặc lỗi CORS.";
          break;
        default:
          errMsg = `Lỗi phát video (Mã: ${mediaError.code}): ${mediaError.message || 'Không rõ nguyên nhân'}`;
      }
    }
    
    setError(`${errMsg} (Xem chi tiết lỗi bảo mật CSP/CORS trong Console F12 của trình duyệt)`);
  };

  const handlePlay = () => {
    console.log("Safe Stream: Video bắt đầu phát thành công.");
  };

  const handleWaiting = () => {
    console.log("Safe Stream: Video đang chờ tải dữ liệu (buffering)...");
  };

  const handleStalled = () => {
    console.warn("Safe Stream: Video bị đứng (stalled) - tốc độ truyền tải chậm.");
  };

  if (!video) {
    return (
      <div className="card" style={{ height: '100%', display: 'flex', flexDirection: 'column', justifyContent: 'center', alignItems: 'center', minHeight: '350px' }}>
        <PlayCircle size={64} className="empty-icon" style={{ opacity: 0.3, marginBottom: '1rem', color: 'var(--text-secondary)' }} />
        <h3 style={{ fontFamily: 'var(--font-title)', fontWeight: 600, fontSize: '1.2rem', marginBottom: '0.5rem' }}>Trình Phát Video An Toàn</h3>
        <p style={{ color: 'var(--text-muted)', fontSize: '0.85rem', textAlign: 'center', maxWidth: '280px' }}>
          Chọn một video từ danh sách để xem trực tiếp trong vùng an toàn (sandbox).
        </p>
      </div>
    );
  }

  return (
    <div className="player-container">
      <div className="player-wrapper" style={{ aspectRatio }}>
        {loading && (
          <div className="loading-screen" style={{ position: 'absolute', inset: 0, background: 'rgba(0,0,0,0.85)', zIndex: 2 }}>
            <Loader2 className="spin" size={32} />
            <p style={{ fontSize: '0.9rem' }}>Đang khởi tạo đường truyền stream bảo mật...</p>
          </div>
        )}

        {error && (
          <div className="loading-screen" style={{ position: 'absolute', inset: 0, background: 'rgba(0,0,0,0.9)', zIndex: 2, padding: '2rem', textAlign: 'center', display: 'flex', flexDirection: 'column', justifyContent: 'center', alignItems: 'center' }}>
            <ShieldAlert size={48} color="var(--color-accent)" style={{ marginBottom: '1rem' }} />
            <h3 style={{ marginBottom: '0.5rem' }}>Lỗi Đường Truyền</h3>
            <p style={{ fontSize: '0.85rem', color: 'var(--text-secondary)', maxWidth: '350px', marginBottom: '1.5rem' }}>{error}</p>
            <button 
              className="btn-primary" 
              onClick={fetchStreamUrl}
              style={{ padding: '0.5rem 1rem', fontSize: '0.85rem', background: 'var(--gradient-brand)', border: 'none', borderRadius: 'var(--radius-sm)', color: 'white', cursor: 'pointer' }}
            >
              Thử tải lại
            </button>
          </div>
        )}

        {streamUrl && (
          <video
            ref={videoRef}
            className="video-element"
            controls
            autoPlay
            playsInline
            controlsList="nodownload" // Prevent default browser download button for security
            id="secure-video-player"
            onLoadedMetadata={(e) => {
              const videoEl = e.currentTarget;
              if (videoEl.videoWidth && videoEl.videoHeight) {
                setAspectRatio(`${videoEl.videoWidth}/${videoEl.videoHeight}`);
              }
            }}
            onError={handleVideoError}
            onPlay={handlePlay}
            onWaiting={handleWaiting}
            onStalled={handleStalled}
          >
            <source src={streamUrl} type="video/mp4" />
            Trình duyệt của bạn không hỗ trợ phát thẻ video HTML5.
          </video>
        )}
      </div>

      <div className="player-header">
        <div>
          <h1 className="player-title">{cleanVideoName(video.original_name)}</h1>
          <p style={{ fontSize: '0.8rem', color: 'var(--text-muted)', marginTop: '0.25rem' }}>
            Lưu trữ với khóa bảo mật: <code style={{ color: 'var(--color-secondary)' }}>{video.key}</code>
          </p>
        </div>
      </div>
    </div>
  );
};
