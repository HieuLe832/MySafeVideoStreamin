import React, { useState, useEffect, useRef } from 'react';
import { Loader2, Trash2 } from 'lucide-react';
import { ActiveDownload } from '../types';
import { videoApi } from '../api/videoApi';
import { formatBytes, cleanVideoName } from '../utils/formatters';

interface DownloadQueueProps {
  onRefreshVideos: () => void;
}

export const DownloadQueue: React.FC<DownloadQueueProps> = ({ onRefreshVideos }) => {
  const [downloads, setDownloads] = useState<ActiveDownload[]>([]);
  const downloadsRef = useRef<ActiveDownload[]>([]);
  downloadsRef.current = downloads;

  useEffect(() => {
    const poll = async () => {
      try {
        const currentTasks = await videoApi.getDownloads();
        
        // Detect if any download task finished successfully since last poll
        const previousTasks = downloadsRef.current;
        let hasCompleted = false;
        
        for (const prevTask of previousTasks) {
          if (
            prevTask.status !== 'Failed' && 
            !currentTasks.some(currTask => currTask.id === prevTask.id)
          ) {
            hasCompleted = true;
            break;
          }
        }
        
        setDownloads(currentTasks);
        
        if (hasCompleted) {
          onRefreshVideos();
        }
      } catch (err) {
        console.error("Lỗi khi tải danh sách tiến trình tải ngầm:", err);
      }
    };

    poll(); // Run immediately on mount
    
    const interval = setInterval(poll, 4000); // Poll every 4 seconds
    return () => clearInterval(interval);
  }, [onRefreshVideos]);

  const handleDismissDownload = async (id: string) => {
    try {
      await videoApi.deleteDownload(id);
      setDownloads(prev => prev.filter(d => d.id !== id));
    } catch (err) {
      console.error("Lỗi khi dọn dẹp tác vụ:", err);
    }
  };

  if (downloads.length === 0) return null;

  return (
    <div className="downloads-container" id="background-downloads-panel">
      <div className="downloads-title">
        <Loader2 className="spin" size={18} style={{ color: 'var(--color-primary)' }} />
        <span>Tiến Trình Tải Video Ngầm ({downloads.length})</span>
      </div>
      
      <div style={{ display: 'flex', flexDirection: 'column', gap: '0.75rem' }}>
        {downloads.map((task) => {
          const isFailed = task.status === 'Failed';
          const isUploading = task.status === 'Uploading';
          const isDownloading = task.status === 'Downloading';
          const isPending = task.status === 'Pending';
          
          let percent = 0;
          if (task.total_bytes && task.total_bytes > 0) {
            percent = Math.round((task.downloaded_bytes * 100) / task.total_bytes);
          }
          
          const statusClass = task.status.toLowerCase();
          const statusLabel = 
            isDownloading ? 'Đang tải' :
            isUploading ? 'Đang đẩy lên Cloud' :
            isPending ? 'Đang chờ (Hàng đợi)' :
            'Thất bại';

          return (
            <div key={task.id} className="download-task-item" id={`download-task-${task.id.substring(0, 8)}`}>
              <div className="download-task-header">
                <span className="download-task-name" title={task.filename}>
                  {cleanVideoName(task.filename)}
                </span>
                <div style={{ display: 'flex', alignItems: 'center', gap: '0.75rem' }}>
                  <span className={`download-task-status ${statusClass}`}>
                    {statusLabel}
                  </span>
                  <button
                    className="btn-icon delete"
                    onClick={() => handleDismissDownload(task.id)}
                    title={isFailed ? "Xóa tác vụ lỗi" : (isPending ? "Hủy hàng đợi" : "Hủy tải và xóa file tạm")}
                    style={{ width: '24px', height: '24px' }}
                  >
                    <Trash2 size={14} />
                  </button>
                </div>
              </div>

              {!isFailed && (
                <div className="download-progress-row">
                  <div className="download-progress-bar-container">
                    <div
                      className="download-progress-bar-value"
                      style={{
                        width: isPending ? '0%' : (task.total_bytes ? `${percent}%` : '100%'),
                        animation: (isPending || task.total_bytes) ? 'none' : 'pulse-glow 2s infinite',
                        opacity: isPending ? 0.3 : 1,
                      }}
                    ></div>
                  </div>
                  <span className="download-progress-text">
                    {isPending ? (
                      'Chờ...'
                    ) : task.total_bytes ? (
                      `${percent}%`
                    ) : (
                      formatBytes(task.downloaded_bytes)
                    )}
                  </span>
                </div>
              )}

              {isFailed && task.error && (
                <div className="download-error-text">
                  Lỗi: {task.error}
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
};
