import { useState, useEffect, useCallback } from 'react';
import { ArrowLeft, Play, Film, RefreshCw } from 'lucide-react';
import { Video } from './types';
import { useAuth } from './hooks/useAuth';
import { videoApi } from './api/videoApi';
import { authApi } from './api/authApi';
import { Login } from './components/Login';
import { Header } from './components/Header';
import { DragDropUpload } from './components/DragDropUpload';
import { VideoList } from './components/VideoList';
import { VideoPlayer } from './components/VideoPlayer';
import { QuotaIndicator } from './components/QuotaIndicator';
import { DownloadQueue } from './components/DownloadQueue';
import { formatBytes, cleanVideoName } from './utils/formatters';

function App() {
  const [videos, setVideos] = useState<Video[]>([]);
  const [totalUsedBytes, setTotalUsedBytes] = useState(0);
  const [maxLimitBytes, setMaxLimitBytes] = useState(9 * 1024 * 1024 * 1024); // 9 GB
  const [activeVideo, setActiveVideo] = useState<Video | null>(null);
  const [loading, setLoading] = useState(false);
  const [currentView, setCurrentView] = useState<'library' | 'upload' | 'player'>('library');
  const [isAuthChecked, setIsAuthChecked] = useState(() => {
    return !localStorage.getItem('access_token');
  });
  const [backendStatus, setBackendStatus] = useState<'checking' | 'connected' | 'disconnected'>('checking');

  // Authentication custom hook
  const {
    isAuthenticated,
    setIsAuthenticated,
    googleClientId,
    authError,
    loading: authLoading,
    handleGoogleLogin,
    handleLogout,
  } = useAuth();

  // Check backend connection status
  const checkConnection = useCallback(async () => {
    setBackendStatus('checking');
    try {
      await authApi.checkHealth();
      setBackendStatus('connected');
    } catch (err) {
      console.error("Backend status check failed:", err);
      setBackendStatus('disconnected');
    }
  }, []);

  // Periodic health check
  useEffect(() => {
    checkConnection();
    const interval = setInterval(checkConnection, 15000);
    return () => clearInterval(interval);
  }, [checkConnection]);

  // Fetches list of video files from backend (supports forceRefresh)
  const fetchVideos = useCallback(async (forceRefresh = false) => {
    setLoading(true);
    try {
      const data = await videoApi.fetchVideos(forceRefresh);
      setVideos(data.videos);
      setTotalUsedBytes(data.total_used_bytes);
      setMaxLimitBytes(data.max_limit_bytes);
      setIsAuthenticated(true);
      setBackendStatus('connected');
    } catch (err: any) {
      console.error(err);
      if (err.response?.status === 401) {
        setIsAuthenticated(false);
        localStorage.removeItem('access_token');
      } else {
        setBackendStatus('disconnected');
      }
    } finally {
      setLoading(false);
    }
  }, [setIsAuthenticated]);

  // Initial fetch of videos
  useEffect(() => {
    const initApp = async () => {
      const hasToken = !!localStorage.getItem('access_token');
      if (hasToken) {
        await fetchVideos(false); // First load does not need force refresh
      }
      setIsAuthChecked(true);
    };
    initApp();
  }, [fetchVideos]);

  // Sync active video with the newly loaded video list
  useEffect(() => {
    if (activeVideo) {
      const matchingVideo = videos.find(v => v.key === activeVideo.key);
      if (matchingVideo) {
        if (
          matchingVideo.original_name !== activeVideo.original_name ||
          matchingVideo.size !== activeVideo.size
        ) {
          setActiveVideo(matchingVideo);
        }
      } else {
        setActiveVideo(null);
        if (currentView === 'player') {
          setCurrentView('library');
        }
      }
    }
  }, [videos, activeVideo, currentView]);

  // Update lists when Google login succeeds
  const onGoogleLoginSuccess = (data: any) => {
    setVideos(data.videos);
    setTotalUsedBytes(data.total_used_bytes);
    setMaxLimitBytes(data.max_limit_bytes);
    setBackendStatus('connected');
  };

  // Clear lists and selected video on logout
  const onLogoutSuccess = () => {
    setVideos([]);
    setActiveVideo(null);
    setCurrentView('library');
  };

  // Smooth scroll to the video player on mobile when a video is selected
  useEffect(() => {
    if (activeVideo && currentView === 'player') {
      window.scrollTo({ top: 0, behavior: 'smooth' });
    }
  }, [activeVideo, currentView]);

  // Handle video card selection
  const handleSelectVideo = (video: Video) => {
    setActiveVideo(video);
    setCurrentView('player');
  };

  // Switch tabs
  const handleViewChange = (view: 'library' | 'upload') => {
    setCurrentView(view);
    if (view === 'library') {
      setActiveVideo(null);
    }
  };

  // Render splash/loading screen if authentication is being checked
  if (!isAuthChecked) {
    return (
      <div style={{ display: 'flex', flexDirection: 'column', justifyContent: 'center', alignItems: 'center', minHeight: '100vh', background: 'var(--bg-primary)' }}>
        <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '1rem' }}>
          <div className="logo-icon" style={{ width: '64px', height: '64px', borderRadius: '20px', display: 'flex', alignItems: 'center', justifyContent: 'center', background: 'var(--gradient-brand)', boxShadow: 'var(--shadow-glow)' }}>
            <Film size={32} style={{ color: 'white' }} />
          </div>
          <h1 style={{ fontSize: '1.75rem', fontFamily: 'var(--font-title)', fontWeight: 800 }}>Sandbox Stream</h1>
          <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', color: 'var(--text-secondary)', fontSize: '0.9rem' }}>
            <RefreshCw size={16} className="spin" />
            <span>Đang kiểm tra đăng nhập...</span>
          </div>
        </div>
      </div>
    );
  }

  // Render lock screen if not authenticated
  if (!isAuthenticated) {
    return (
      <Login
        googleClientId={googleClientId}
        loading={authLoading}
        authError={authError}
        onGoogleLogin={(token) => handleGoogleLogin(token, onGoogleLoginSuccess)}
        backendStatus={backendStatus}
        onCheckConnection={checkConnection}
      />
    );
  }

  return (
    <div className="container">
      {/* App Navigation Header */}
      <Header
        loading={loading}
        currentView={currentView}
        onViewChange={handleViewChange}
        onRefresh={() => fetchVideos(true)} // Force refresh from R2 on manual reload
        onLogout={() => handleLogout(onLogoutSuccess)}
        backendStatus={backendStatus}
        onCheckConnection={checkConnection}
      />

      {/* Main UI Layout View Router */}
      <main style={{ minWidth: 0, width: '100%' }}>
        {currentView === 'library' && (
          <VideoList
            videos={videos}
            loading={loading}
            activeVideo={activeVideo}
            onSelectVideo={handleSelectVideo}
            onRefresh={() => fetchVideos(true)} // Force refresh from R2 on list mutations (rename/delete)
          />
        )}

        {currentView === 'upload' && (
          <div style={{ maxWidth: '680px', margin: '0 auto', display: 'flex', flexDirection: 'column', gap: '1.5rem', width: '100%' }}>
            <QuotaIndicator totalUsedBytes={totalUsedBytes} maxLimitBytes={maxLimitBytes} />
            <DragDropUpload onUploadSuccess={() => fetchVideos(true)} /> {/* Force refresh on upload */}
            <DownloadQueue onRefreshVideos={() => fetchVideos(true)} /> {/* Force refresh on background download finished */}
          </div>
        )}

        {currentView === 'player' && (
          <div className="player-page-grid">
            {/* Left Column: Player & Meta */}
            <div style={{ display: 'flex', flexDirection: 'column', gap: '0.75rem', minWidth: 0 }}>
              <button 
                className="btn-back" 
                onClick={() => {
                  setCurrentView('library');
                  setActiveVideo(null);
                }}
              >
                <ArrowLeft size={14} /> Quay lại thư viện
              </button>
              <VideoPlayer video={activeVideo} />
            </div>

            {/* Right Column: Suggested playlist */}
            <div className="watch-next-container">
              <h3 className="watch-next-header">Video khác ({videos.length})</h3>
              <div className="watch-next-list">
                {videos.map((v) => {
                  const isPlaying = v.key === activeVideo?.key;
                  return (
                    <div
                      key={v.key}
                      className={`watch-next-item ${isPlaying ? 'active' : ''}`}
                      onClick={() => {
                        setActiveVideo(v);
                      }}
                    >
                      <div className="watch-next-thumbnail-wrapper">
                        <div className="video-thumbnail-wrapper">
                          <div className="video-thumbnail-placeholder">
                            <Play 
                              size={14} 
                              fill={isPlaying ? 'var(--color-secondary)' : 'none'} 
                              style={{ 
                                color: isPlaying ? 'var(--color-secondary)' : 'var(--text-muted)',
                                opacity: isPlaying ? 1 : 0.4 
                              }} 
                            />
                          </div>
                        </div>
                      </div>
                      <div className="watch-next-info">
                        <div className="watch-next-title" title={cleanVideoName(v.original_name)}>
                          {cleanVideoName(v.original_name)}
                        </div>
                        <div className="watch-next-meta">
                          {formatBytes(v.size)}
                        </div>
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>
          </div>
        )}
      </main>
    </div>
  );
}

export default App;
