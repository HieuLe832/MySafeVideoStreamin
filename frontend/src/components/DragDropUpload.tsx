import React, { useState, useRef, useEffect } from 'react';
import axios from 'axios';
import { UploadCloud, FileVideo, AlertCircle, CheckCircle, Link, Globe, RefreshCw, Clock, Check } from 'lucide-react';
import { videoApi } from '../api/videoApi';
import { formatBytes } from '../utils/formatters';
import { isVideoFileSignature } from '../utils/fileValidation';

interface DragDropUploadProps {
  onUploadSuccess: () => void;
}

interface UploadQueueItem {
  id: string;
  file: File;
  progress: number;
  speed: string;
  status: 'pending' | 'uploading' | 'completed' | 'failed';
  error?: string;
  abortController?: AbortController;
}

export const DragDropUpload: React.FC<DragDropUploadProps> = ({ onUploadSuccess }) => {
  const [activeTab, setActiveTab] = useState<'file' | 'url'>('file');
  const [videoUrl, setVideoUrl] = useState('');
  const [urlUploading, setUrlUploading] = useState(false);

  const [isDragging, setIsDragging] = useState(false);
  const [queue, setQueue] = useState<UploadQueueItem[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const processingIdsRef = useRef<Set<string>>(new Set());

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    setIsDragging(true);
  };

  const handleDragLeave = () => {
    setIsDragging(false);
  };

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    setIsDragging(false);
    
    if (e.dataTransfer.files && e.dataTransfer.files.length > 0) {
      addFilesToQueue(e.dataTransfer.files);
    }
  };

  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    if (e.target.files && e.target.files.length > 0) {
      addFilesToQueue(e.target.files);
    }
  };

  const triggerFileInput = () => {
    fileInputRef.current?.click();
  };

  const addFilesToQueue = (files: FileList) => {
    setError(null);
    setSuccess(null);

    const newItems: UploadQueueItem[] = [];
    for (let i = 0; i < files.length; i++) {
      const file = files[i];
      const id = `${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
      newItems.push({
        id,
        file,
        progress: 0,
        speed: 'Chờ...',
        status: 'pending',
      });
    }

    setQueue((prev) => [...prev, ...newItems]);
    if (fileInputRef.current) {
      fileInputRef.current.value = '';
    }
  };

  const processQueueItem = async (itemId: string) => {
    let itemToUpload = queue.find((item) => item.id === itemId);
    if (!itemToUpload) {
      processingIdsRef.current.delete(itemId);
      return;
    }

    const file = itemToUpload.file;
    const controller = new AbortController();

    setQueue((prev) =>
      prev.map((item) =>
        item.id === itemId
          ? { ...item, status: 'uploading', abortController: controller, speed: 'Đang chuẩn bị...' }
          : item
      )
    );

    try {
      const maxFileSize = 2 * 1024 * 1024 * 1024; // 2 GB
      if (file.size > maxFileSize) {
        throw new Error(`File quá lớn (${formatBytes(file.size)}). Giới hạn tối đa là 2 GB.`);
      }

      const isValidVideo = await isVideoFileSignature(file);
      if (!isValidVideo) {
        throw new Error('Chữ ký Magic Bytes không hợp lệ (không phải định dạng video được hỗ trợ).');
      }

      const response = await videoApi.getUploadUrl(file.name, file.size);
      const { upload_url } = response;
      const startTime = Date.now();

      const cleanAxios = axios.create({
        baseURL: '',
      });

      await cleanAxios.put(upload_url, file, {
        headers: {
          'Content-Type': file.type || 'video/mp4',
        },
        signal: controller.signal,
        onUploadProgress: (progressEvent) => {
          if (progressEvent.total) {
            const percent = Math.round((progressEvent.loaded * 100) / progressEvent.total);
            const elapsedTimeSec = (Date.now() - startTime) / 1000;
            let speedText = 'Đang tính...';
            if (elapsedTimeSec > 0) {
              const speedBytesPerSec = progressEvent.loaded / elapsedTimeSec;
              speedText = `${formatBytes(speedBytesPerSec)}/s`;
            }

            setQueue((prev) =>
              prev.map((item) =>
                item.id === itemId
                  ? { ...item, progress: percent, speed: speedText }
                  : item
              )
            );
          }
        },
      });

      setQueue((prev) =>
        prev.map((item) =>
          item.id === itemId
            ? { ...item, status: 'completed', progress: 100, speed: 'Xong' }
            : item
        )
      );

      onUploadSuccess();
    } catch (err: any) {
      let errorMessage = 'Có lỗi xảy ra khi tải lên file.';
      if (axios.isCancel(err)) {
        errorMessage = 'Đã hủy tải lên.';
      } else {
        console.error('Lỗi chi tiết khi upload:', err);
        errorMessage = err.response?.data?.error || err.message || errorMessage;
        if (err.message === 'Network Error') {
          errorMessage = 'Lỗi mạng hoặc CORS trên R2. Hãy kiểm tra lại cấu hình CORS.';
        }
      }

      setQueue((prev) =>
        prev.map((item) =>
          item.id === itemId
            ? { ...item, status: 'failed', error: errorMessage }
            : item
        )
      );
    } finally {
      processingIdsRef.current.delete(itemId);
    }
  };

  useEffect(() => {
    const activeUpload = queue.find((item) => item.status === 'uploading');
    if (activeUpload) {
      return;
    }

    const nextPending = queue.find((item) => item.status === 'pending');
    if (nextPending && !processingIdsRef.current.has(nextPending.id)) {
      processingIdsRef.current.add(nextPending.id);
      processQueueItem(nextPending.id);
    }
  }, [queue]);

  const handleCancelItem = (itemId: string) => {
    setQueue((prev) => {
      const item = prev.find((i) => i.id === itemId);
      if (item && item.status === 'uploading' && item.abortController) {
        item.abortController.abort();
      }
      return prev.map((i) => i.id === itemId ? { ...i, status: 'failed', error: 'Đã hủy tải lên.' } : i);
    });
  };

  const handleRemoveItem = (itemId: string) => {
    setQueue((prev) => {
      const item = prev.find((i) => i.id === itemId);
      if (item && item.status === 'uploading' && item.abortController) {
        item.abortController.abort();
      }
      return prev.filter((i) => i.id !== itemId);
    });
  };

  const clearQueue = () => {
    queue.forEach((item) => {
      if (item.status === 'uploading' && item.abortController) {
        item.abortController.abort();
      }
    });
    setQueue([]);
    processingIdsRef.current.clear();
  };

  const clearCompletedAndFailed = () => {
    setQueue((prev) => prev.filter((item) => item.status === 'pending' || item.status === 'uploading'));
  };

  const handleUrlSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!videoUrl.trim()) return;

    setError(null);
    setSuccess(null);
    setUrlUploading(true);

    try {
      await videoApi.uploadFromUrl(videoUrl);
      setSuccess('Đã gửi yêu cầu tải video! Tiến trình đang chạy ngầm trên máy chủ, bạn có thể tắt máy tính.');
      setVideoUrl('');
      onUploadSuccess();
    } catch (err: any) {
      console.error(err);
      setError(err.response?.data?.error || err.message || 'Có lỗi xảy ra khi gửi yêu cầu tải video.');
    } finally {
      setUrlUploading(false);
    }
  };

  return (
    <div className="card" id="upload-card">
      <h2 className="card-title">
        <UploadCloud size={20} className="gradient-text" /> Tải Lên Video
      </h2>

      <div className="upload-tabs">
        <button
          className={`upload-tab-btn ${activeTab === 'file' ? 'active' : ''}`}
          onClick={() => {
            setError(null);
            setSuccess(null);
            setActiveTab('file');
          }}
          disabled={queue.some(i => i.status === 'uploading') || urlUploading}
        >
          <UploadCloud size={16} /> Tải file từ máy
        </button>
        <button
          className={`upload-tab-btn ${activeTab === 'url' ? 'active' : ''}`}
          onClick={() => {
            setError(null);
            setSuccess(null);
            setActiveTab('url');
          }}
          disabled={queue.some(i => i.status === 'uploading') || urlUploading}
          id="url-upload-tab"
        >
          <Link size={16} /> Tải từ liên kết
        </button>
      </div>

      {activeTab === 'file' ? (
        <div
          className={`upload-zone ${isDragging ? 'dragging' : ''}`}
          onDragOver={handleDragOver}
          onDragLeave={handleDragLeave}
          onDrop={handleDrop}
          onClick={triggerFileInput}
          id="drop-zone"
        >
          <input
            type="file"
            className="file-input"
            ref={fileInputRef}
            onChange={handleFileChange}
            accept="video/*"
            id="video-file-input"
            multiple
          />
          <div className="upload-icon-container">
            <UploadCloud size={32} />
          </div>
          <div className="upload-text-main">Kéo & thả video hoặc click để chọn</div>
          <div className="upload-text-sub">Hỗ trợ chọn cùng lúc nhiều file video (Tối đa 2 GB mỗi file)</div>
        </div>
      ) : (
        <form onSubmit={handleUrlSubmit} className="url-upload-form" id="url-upload-form">
          <div className="form-group">
            <label htmlFor="video-url-input" className="input-label">
              Liên kết tải video trực tiếp (Direct Link)
            </label>
            <input
              type="url"
              id="video-url-input"
              className="text-input"
              placeholder="https://example.com/movie.mp4"
              value={videoUrl}
              onChange={(e) => setVideoUrl(e.target.value)}
              disabled={urlUploading}
              required
            />
          </div>
          <button type="submit" className="btn-primary" disabled={urlUploading || !videoUrl.trim()} id="url-submit-button">
            {urlUploading ? (
              <>
                <RefreshCw size={18} className="spin" /> Đang gửi...
              </>
            ) : (
              <>
                <Globe size={18} /> Bắt đầu tải video
              </>
            )}
          </button>
        </form>
      )}

      {queue.length > 0 && (
        <div className="queue-container" style={{ marginTop: '1.5rem', display: 'flex', flexDirection: 'column', gap: '0.75rem' }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <span style={{ fontSize: '0.85rem', fontWeight: 600, color: 'var(--text-secondary)' }}>
              Hàng đợi tải lên ({queue.filter(i => i.status === 'completed').length}/{queue.length} tệp)
            </span>
            <div style={{ display: 'flex', gap: '0.5rem' }}>
              <button
                type="button"
                className="upload-tab-btn"
                onClick={clearCompletedAndFailed}
                style={{ padding: '4px 8px', fontSize: '0.75rem', borderRadius: 'var(--radius-sm)', border: '1px solid var(--border-color)', background: 'rgba(255,255,255,0.02)', cursor: 'pointer' }}
                disabled={!queue.some(i => i.status === 'completed' || i.status === 'failed')}
              >
                Dọn dẹp
              </button>
              <button
                type="button"
                className="upload-tab-btn"
                onClick={clearQueue}
                style={{ padding: '4px 8px', fontSize: '0.75rem', borderRadius: 'var(--radius-sm)', border: '1px solid rgba(244,63,94,0.2)', background: 'rgba(244,63,94,0.05)', color: 'var(--color-accent)', cursor: 'pointer' }}
              >
                Hủy tất cả
              </button>
            </div>
          </div>

          <div className="queue-list" style={{ maxHeight: '300px', overflowY: 'auto', display: 'flex', flexDirection: 'column', gap: '0.5rem', paddingRight: '4px' }}>
            {queue.map((item) => (
              <div 
                key={item.id} 
                className="upload-status" 
                style={{ 
                  margin: 0, 
                  background: 'rgba(255, 255, 255, 0.02)',
                  borderColor: item.status === 'uploading' ? 'var(--border-glow)' : 'var(--border-color)'
                }}
              >
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: item.status === 'uploading' ? '0.5rem' : '0', gap: '1rem' }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', minWidth: 0 }}>
                    <FileVideo 
                      size={16} 
                      style={{ 
                        color: item.status === 'completed' ? 'var(--color-success)' : item.status === 'failed' ? 'var(--color-accent)' : 'var(--color-primary)',
                        flexShrink: 0 
                      }} 
                    />
                    <span 
                      style={{ 
                        fontSize: '0.85rem', 
                        fontWeight: 600, 
                        whiteSpace: 'nowrap', 
                        overflow: 'hidden', 
                        textOverflow: 'ellipsis',
                        color: item.status === 'completed' ? 'var(--text-muted)' : 'var(--text-primary)'
                      }} 
                      title={item.file.name}
                    >
                      {item.file.name}
                    </span>
                  </div>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', flexShrink: 0 }}>
                    <span style={{ fontSize: '0.8rem', color: 'var(--text-muted)' }}>
                      {formatBytes(item.file.size)}
                    </span>
                    {item.status === 'uploading' && (
                      <span style={{ fontSize: '0.8rem', color: 'var(--color-secondary)', fontWeight: 700 }}>
                        {item.progress}%
                      </span>
                    )}
                    {item.status === 'completed' && (
                      <span style={{ fontSize: '0.8rem', color: 'var(--color-success)', fontWeight: 600, display: 'flex', alignItems: 'center', gap: '2px' }}>
                        <Check size={12} /> Xong
                      </span>
                    )}
                    {item.status === 'pending' && (
                      <span style={{ fontSize: '0.8rem', color: 'var(--text-muted)', display: 'flex', alignItems: 'center', gap: '2px' }}>
                        <Clock size={12} /> Chờ
                      </span>
                    )}
                  </div>
                </div>

                {item.status === 'uploading' && (
                  <>
                    <div className="upload-progress-bar-bg" style={{ marginBottom: '0.4rem' }}>
                      <div
                        className="upload-progress-bar-fill"
                        style={{ width: `${item.progress}%` }}
                      ></div>
                    </div>
                    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', fontSize: '0.75rem', color: 'var(--text-muted)' }}>
                      <span>Tốc độ: {item.speed}</span>
                      <button
                        type="button"
                        onClick={() => handleCancelItem(item.id)}
                        style={{
                          background: 'none',
                          border: 'none',
                          color: 'var(--color-accent)',
                          cursor: 'pointer',
                          fontWeight: 600,
                          padding: '2px 6px',
                        }}
                      >
                        Hủy
                      </button>
                    </div>
                  </>
                )}

                {item.status === 'failed' && (
                  <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', fontSize: '0.75rem', color: 'var(--color-accent)', gap: '1rem', marginTop: '0.25rem' }}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '0.25rem', minWidth: 0 }}>
                      <AlertCircle size={12} style={{ flexShrink: 0 }} />
                      <span style={{ whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }} title={item.error}>
                        Lỗi: {item.error}
                      </span>
                    </div>
                    <button
                      type="button"
                      onClick={() => handleRemoveItem(item.id)}
                      style={{
                        background: 'none',
                        border: 'none',
                        color: 'var(--text-muted)',
                        cursor: 'pointer',
                        fontWeight: 600,
                        padding: '2px 6px',
                        flexShrink: 0
                      }}
                    >
                      Xóa
                    </button>
                  </div>
                )}

                {item.status === 'completed' && (
                  <div style={{ display: 'flex', justifyContent: 'flex-end', marginTop: '0.25rem' }}>
                    <button
                      type="button"
                      onClick={() => handleRemoveItem(item.id)}
                      style={{
                        background: 'none',
                        border: 'none',
                        color: 'var(--text-muted)',
                        cursor: 'pointer',
                        fontSize: '0.7rem',
                        padding: '0 4px',
                      }}
                    >
                      Xóa khỏi danh sách
                    </button>
                  </div>
                )}
                
                {item.status === 'pending' && (
                  <div style={{ display: 'flex', justifyContent: 'flex-end', marginTop: '0.25rem' }}>
                    <button
                      type="button"
                      onClick={() => handleRemoveItem(item.id)}
                      style={{
                        background: 'none',
                        border: 'none',
                        color: 'var(--color-accent)',
                        cursor: 'pointer',
                        fontSize: '0.7rem',
                        padding: '0 4px',
                      }}
                    >
                      Hủy bỏ
                    </button>
                  </div>
                )}
              </div>
            ))}
          </div>
        </div>
      )}

      {error && (
        <div className="alert-box error" id="upload-error-alert">
          <AlertCircle size={16} style={{ flexShrink: 0 }} />
          <div>{error}</div>
        </div>
      )}

      {success && (
        <div className="alert-box success" id="upload-success-alert">
          <CheckCircle size={16} style={{ flexShrink: 0 }} />
          <div>{success}</div>
        </div>
      )}
    </div>
  );
};
