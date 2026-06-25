import React, { useState, useEffect, useCallback } from 'react';
import { Play, Trash2, Pencil, FileVideo, AlertCircle, Loader2, Folder, ArrowLeft } from 'lucide-react';
import { Video } from '../types';
import { videoApi } from '../api/videoApi';
import { formatBytes, cleanVideoName } from '../utils/formatters';

interface VideoThumbnailProps {
  videoKey: string;
  isWorking: boolean;
  isActive: boolean;
  onLoadStart?: (key: string) => void;
  onLoadEnd?: (key: string) => void;
}

const VideoThumbnail: React.FC<VideoThumbnailProps> = ({ 
  videoKey, 
  isWorking, 
  isActive,
  onLoadStart,
  onLoadEnd
}) => {
  const [hasLoaded, setHasLoaded] = useState(false);
  const [hasError, setHasError] = useState(false);
  const [isLoadingStarted, setIsLoadingStarted] = useState(false);

  useEffect(() => {
    setHasLoaded(false);
    setHasError(false);
    
    if (!isWorking && onLoadStart) {
      onLoadStart(videoKey);
      setIsLoadingStarted(true);
    }

    return () => {
      if (onLoadEnd) {
        onLoadEnd(videoKey);
      }
      setIsLoadingStarted(false);
    };
  }, [videoKey, isWorking, onLoadStart, onLoadEnd]);

  useEffect(() => {
    if (isWorking && isLoadingStarted) {
      if (onLoadEnd) {
        onLoadEnd(videoKey);
      }
      setIsLoadingStarted(false);
    }
  }, [isWorking, isLoadingStarted, videoKey, onLoadEnd]);

  const apiBaseUrl = import.meta.env.VITE_API_BASE_URL || '';
  const token = localStorage.getItem('access_token');
  const encodedKey = encodeURIComponent(videoKey);
  const thumbnailUrl = `${apiBaseUrl}/api/videos/${encodedKey}/thumbnail?token=${encodeURIComponent(token || '')}`;

  const handleLoadSuccess = () => {
    setHasLoaded(true);
    if (isLoadingStarted && onLoadEnd) {
      onLoadEnd(videoKey);
      setIsLoadingStarted(false);
    }
  };

  const handleLoadError = () => {
    setHasError(true);
    setHasLoaded(false);
    if (isLoadingStarted && onLoadEnd) {
      onLoadEnd(videoKey);
      setIsLoadingStarted(false);
    }
  };

  return (
    <div className="video-thumbnail-wrapper" style={{ position: 'relative', width: '100%', height: '100%' }}>
      {isWorking ? (
        <Loader2 className="spin" size={20} style={{ color: 'var(--text-secondary)' }} />
      ) : (
        <>
          <div className="video-thumbnail-overlay" style={{ opacity: hasLoaded ? 0.35 : 0, transition: 'opacity 0.2s ease' }}>
            <Play size={22} fill={isActive ? 'currentColor' : 'white'} style={{ color: isActive ? 'var(--color-primary)' : 'white' }} />
          </div>
          
          {!hasError && (
            <img
              src={thumbnailUrl}
              alt="Video thumbnail"
              className="video-thumbnail-element"
              style={{ 
                width: '100%', 
                height: '100%', 
                objectFit: 'cover',
                opacity: hasLoaded ? 1 : 0,
                transition: 'opacity 0.3s ease'
              }}
              onLoad={handleLoadSuccess}
              onError={handleLoadError}
            />
          )}

          {(!hasLoaded || hasError) && (
            <div className="video-thumbnail-placeholder" style={{ position: 'absolute', inset: 0, display: 'flex', alignItems: 'center', justifyContent: 'center', background: 'rgba(255,255,255,0.02)' }}>
              <FileVideo size={28} style={{ opacity: 0.2 }} />
            </div>
          )}
        </>
      )}
    </div>
  );
};

interface VideoListProps {
  videos: Video[];
  loading: boolean;
  activeVideo: Video | null;
  onSelectVideo: (video: Video) => void;
  onRefresh: () => void;
}

interface GroupedData {
  albums: { [albumName: string]: Video[] };
  singleVideos: Video[];
}

const groupVideos = (videos: Video[]): GroupedData => {
  const separators = [' - ', ' | ', ' : '];
  const prefixCounts: { [prefix: string]: number } = {};
  const videoPrefixes: { [key: string]: { prefix: string; cleanTitle: string } } = {};
  
  videos.forEach((video) => {
    const cleanName = cleanVideoName(video.original_name);
    let foundSeparator = false;
    
    for (const sep of separators) {
      const parts = cleanName.split(sep);
      if (parts.length > 1) {
        const prefix = parts[0].trim();
        const cleanTitle = parts.slice(1).join(sep).trim();
        if (prefix && cleanTitle) {
          videoPrefixes[video.key] = { prefix, cleanTitle };
          prefixCounts[prefix] = (prefixCounts[prefix] || 0) + 1;
          foundSeparator = true;
          break;
        }
      }
    }
    
    if (!foundSeparator) {
      videoPrefixes[video.key] = { prefix: '', cleanTitle: cleanName };
    }
  });
  
  const albums: { [albumName: string]: Video[] } = {};
  const singleVideos: Video[] = [];
  
  videos.forEach((video) => {
    const info = videoPrefixes[video.key];
    if (info.prefix && prefixCounts[info.prefix] >= 2) {
      if (!albums[info.prefix]) {
        albums[info.prefix] = [];
      }
      albums[info.prefix].push(video);
    } else {
      singleVideos.push(video);
    }
  });
  
  // Sort albums alphabetically and videos within by upload date (newest first)
  Object.keys(albums).forEach((albumName) => {
    albums[albumName].sort((a, b) => b.uploaded_at.localeCompare(a.uploaded_at));
  });

  return { albums, singleVideos };
};

export const VideoList: React.FC<VideoListProps> = ({
  videos,
  loading,
  activeVideo,
  onSelectVideo,
  onRefresh,
}) => {
  const [activeAlbum, setActiveAlbum] = useState<string | null>(null);
  const [deletingKey, setDeletingKey] = useState<string | null>(null);
  const [renamingKey, setRenamingKey] = useState<string | null>(null);
  const [operationError, setOperationError] = useState<string | null>(null);
  const [loadingThumbs, setLoadingThumbs] = useState<Set<string>>(new Set());

  // Reset loading thumbnails when view or video list changes
  useEffect(() => {
    setLoadingThumbs(new Set());
  }, [activeAlbum, videos]);

  const handleThumbStart = useCallback((key: string) => {
    setLoadingThumbs(prev => {
      if (prev.has(key)) return prev;
      const next = new Set(prev);
      next.add(key);
      return next;
    });
  }, []);

  const handleThumbEnd = useCallback((key: string) => {
    setLoadingThumbs(prev => {
      if (!prev.has(key)) return prev;
      const next = new Set(prev);
      next.delete(key);
      return next;
    });
  }, []);

  const { albums, singleVideos } = groupVideos(videos);

  // Auto fallback if album is empty or deleted
  useEffect(() => {
    if (activeAlbum && (!albums[activeAlbum] || albums[activeAlbum].length === 0)) {
      setActiveAlbum(null);
    }
  }, [videos, activeAlbum, albums]);

  const handleDelete = async (e: React.MouseEvent, key: string, originalName: string) => {
    e.stopPropagation();
    
    const cleanedName = cleanVideoName(originalName);
    if (!window.confirm(`Bạn có chắc chắn muốn xóa video "${cleanedName}" không?`)) {
      return;
    }

    setDeletingKey(key);
    setOperationError(null);

    try {
      await videoApi.deleteVideo(key);
      onRefresh();
    } catch (err: any) {
      console.error(err);
      setOperationError(`Lỗi xóa file: ${err.response?.data?.error || err.message}`);
    } finally {
      setDeletingKey(null);
    }
  };

  const handleRename = async (e: React.MouseEvent, key: string, originalName: string) => {
    e.stopPropagation();

    const cleanedName = cleanVideoName(originalName);
    const newName = window.prompt(`Nhập tên mới cho video "${cleanedName}":`, cleanedName);
    if (newName === null) {
      return;
    }

    const trimmedNewName = newName.trim();
    if (!trimmedNewName) {
      alert("Tên file không được trống!");
      return;
    }

    const dotIndex = originalName.lastIndexOf('.');
    const ext = dotIndex !== -1 ? originalName.slice(dotIndex) : '';
    let finalNewName = trimmedNewName;
    if (ext && !trimmedNewName.toLowerCase().endsWith(ext.toLowerCase())) {
      finalNewName = trimmedNewName + ext;
    }

    if (finalNewName === originalName) {
      return;
    }

    setRenamingKey(key);
    setOperationError(null);

    try {
      await videoApi.renameVideo(key, finalNewName);
      onRefresh();
    } catch (err: any) {
      console.error(err);
      setOperationError(`Lỗi đổi tên file: ${err.response?.data?.error || err.message}`);
    } finally {
      setRenamingKey(null);
    }
  };

  const isInsideAlbum = activeAlbum !== null;
  const displayedVideos = isInsideAlbum ? albums[activeAlbum!] || [] : singleVideos;

  return (
    <div className="video-grid-container" id="library-grid-container">
      {/* Library Header */}
      <div className="library-header" style={{ display: 'flex', flexDirection: 'column', alignItems: 'flex-start', gap: '0.75rem', width: '100%', marginBottom: '1.5rem' }}>
        {isInsideAlbum ? (
          <div style={{ display: 'flex', flexDirection: 'column', gap: '0.5rem', width: '100%' }}>
            <button 
              className="upload-tab-btn" 
              onClick={() => setActiveAlbum(null)}
              style={{ display: 'flex', alignItems: 'center', gap: '0.35rem', padding: '6px 12px', fontSize: '0.85rem', width: 'fit-content', border: '1px solid var(--border-color)', borderRadius: 'var(--radius-sm)', background: 'rgba(255,255,255,0.02)', cursor: 'pointer' }}
            >
              <ArrowLeft size={14} /> Quay lại danh mục chính
            </button>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', width: '100%', marginTop: '0.25rem' }}>
              <h2 className="card-title" style={{ marginBottom: 0, fontSize: '1.35rem' }}>
                Album: <span className="gradient-text">{activeAlbum}</span>
              </h2>
              <span className="library-count" id="video-count-badge">
                {displayedVideos.length} video
              </span>
            </div>
          </div>
        ) : (
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', width: '100%' }}>
            <h2 className="card-title" style={{ marginBottom: 0 }}>
              <FileVideo size={20} className="gradient-text" /> Thư Viện Video
            </h2>
            <span className="library-count" id="video-count-badge">
              {videos.length} video
            </span>
          </div>
        )}
      </div>

      {/* Dynamic Thumbnail Loading Status Bar */}
      {loadingThumbs.size > 0 && (
        <div 
          className="thumbnail-loading-status"
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: '0.65rem',
            padding: '10px 16px',
            background: 'rgba(249, 115, 22, 0.06)',
            border: '1px solid rgba(249, 115, 22, 0.25)',
            borderRadius: 'var(--radius-md)',
            color: 'var(--color-primary)',
            fontSize: '0.85rem',
            marginBottom: '1.5rem',
            width: '100%',
            boxShadow: '0 4px 12px rgba(249, 115, 22, 0.05)',
            transition: 'all 0.3s ease',
          }}
        >
          <Loader2 className="spin" size={16} style={{ color: 'var(--color-primary)' }} />
          <span style={{ fontWeight: 500 }}>
            Đang tải ảnh thu nhỏ cho {loadingThumbs.size} video...
          </span>
        </div>
      )}

      {operationError && (
        <div className="alert-box error" style={{ marginBottom: '1.5rem' }}>
          <AlertCircle size={16} style={{ flexShrink: 0 }} />
          <div>{operationError}</div>
        </div>
      )}

      {loading && videos.length === 0 ? (
        <div className="empty-library">
          <Loader2 className="spin" size={32} />
          <p>Đang tải danh sách video...</p>
        </div>
      ) : videos.length === 0 ? (
        <div className="empty-library" id="empty-library-state">
          <FileVideo size={48} className="empty-icon" />
          <p>Chưa có video nào được tải lên.</p>
          <p style={{ fontSize: '0.8rem', color: 'var(--text-muted)', maxWidth: '320px' }}>
            Hãy chọn tab "Tải lên / Tải về" để thêm video trực tiếp hoặc từ liên kết.
          </p>
        </div>
      ) : (
        <div className="video-grid" id="video-list-container">
          {/* Render Album Folders if in main library view */}
          {!isInsideAlbum && Object.entries(albums).map(([albumName, albumVideos]) => {
            const albumSize = albumVideos.reduce((acc, v) => acc + v.size, 0);
            return (
              <div
                key={albumName}
                className="video-card"
                onClick={() => setActiveAlbum(albumName)}
                style={{ borderStyle: 'dashed', borderColor: 'rgba(249, 115, 22, 0.3)' }}
              >
                <div 
                  className="video-card-thumbnail-container" 
                  style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', background: 'rgba(255, 255, 255, 0.01)' }}
                >
                  <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '0.5rem', color: 'var(--color-primary)' }}>
                    <Folder size={44} style={{ filter: 'drop-shadow(0 4px 12px rgba(249, 115, 22, 0.25))' }} />
                    <span style={{ fontSize: '0.75rem', color: 'var(--text-muted)', fontWeight: 600 }}>Thư mục Album</span>
                  </div>
                </div>
                
                <div className="video-card-info">
                  <div className="video-card-title" title={albumName} style={{ height: 'auto', fontWeight: 700, fontSize: '0.95rem', marginBottom: '0.25rem' }}>
                    {albumName}
                  </div>
                  
                  <div className="video-card-metadata" style={{ marginTop: '0.25rem' }}>
                    <span>{albumVideos.length} video</span>
                    <span>{formatBytes(albumSize)}</span>
                  </div>
                </div>
              </div>
            );
          })}

          {/* Render Videos (Inside Album, or Single Videos) */}
          {displayedVideos.map((video) => {
            const isActive = activeVideo?.key === video.key;
            const isDeleting = deletingKey === video.key;
            const isRenaming = renamingKey === video.key;
            const isWorking = isDeleting || isRenaming;

            const formattedDate = new Date(video.uploaded_at).toLocaleString('vi-VN', {
              hour: '2-digit',
              minute: '2-digit',
              day: '2-digit',
              month: '2-digit',
              year: 'numeric',
            });

            // If we are inside an album, clean prefix from the display title
            let displayTitle = cleanVideoName(video.original_name);
            if (isInsideAlbum && displayTitle.startsWith(activeAlbum + ' - ')) {
              displayTitle = displayTitle.substring(activeAlbum.length + 3);
            } else if (isInsideAlbum && displayTitle.startsWith(activeAlbum + ' | ')) {
              displayTitle = displayTitle.substring(activeAlbum.length + 3);
            } else if (isInsideAlbum && displayTitle.startsWith(activeAlbum + ' : ')) {
              displayTitle = displayTitle.substring(activeAlbum.length + 3);
            }

            return (
              <div
                key={video.key}
                className={`video-card ${isActive ? 'active' : ''}`}
                onClick={() => {
                  if (!isWorking) {
                    onSelectVideo(video);
                  }
                }}
                id={`video-card-${video.key.substring(0, 8)}`}
                style={{ cursor: isWorking ? 'not-allowed' : 'pointer' }}
              >
                <div className="video-card-thumbnail-container">
                  <VideoThumbnail
                    videoKey={video.key}
                    isWorking={isWorking}
                    isActive={isActive}
                    onLoadStart={handleThumbStart}
                    onLoadEnd={handleThumbEnd}
                  />
                </div>
                
                <div className="video-card-info">
                  <div className="video-card-title" title={displayTitle}>
                    {displayTitle}
                  </div>
                  
                  <div className="video-card-metadata">
                    <div style={{ display: 'flex', flexDirection: 'column', gap: '0.125rem' }}>
                      <span>{formatBytes(video.size)}</span>
                      <span style={{ fontSize: '0.7rem', color: 'var(--text-muted)' }}>
                        {formattedDate !== 'Invalid Date' ? formattedDate : video.uploaded_at}
                      </span>
                    </div>

                    <div className="video-card-actions">
                      <button
                        className="btn-icon edit"
                        title="Đổi tên video"
                        onClick={(e) => handleRename(e, video.key, video.original_name)}
                        disabled={isWorking}
                        id={`rename-btn-${video.key.substring(0, 8)}`}
                        style={{ width: '28px', height: '28px' }}
                      >
                        <Pencil size={13} />
                      </button>
                      <button
                        className="btn-icon delete"
                        title="Xóa video"
                        onClick={(e) => handleDelete(e, video.key, video.original_name)}
                        disabled={isWorking}
                        id={`delete-btn-${video.key.substring(0, 8)}`}
                        style={{ width: '28px', height: '28px' }}
                      >
                        <Trash2 size={13} />
                      </button>
                    </div>
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
};


