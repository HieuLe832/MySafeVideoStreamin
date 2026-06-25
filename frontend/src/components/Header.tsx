import { RefreshCw, LogOut, Film, UploadCloud } from 'lucide-react';

interface HeaderProps {
  loading: boolean;
  currentView: 'library' | 'upload' | 'player';
  onViewChange: (view: 'library' | 'upload') => void;
  onRefresh: () => void;
  onLogout: () => void;
  backendStatus: 'checking' | 'connected' | 'disconnected';
  onCheckConnection: () => void;
}

export const Header: React.FC<HeaderProps> = ({
  loading,
  currentView,
  onViewChange,
  onRefresh,
  onLogout,
  backendStatus,
  onCheckConnection,
}) => {
  const hasToken = !!localStorage.getItem('access_token');

  return (
    <header className="app-header">
      <div className="header-top-row">
        <div className="logo-section">
          <div className="logo-icon">
            <Film size={22} />
          </div>
          <div>
            <div className="logo-text">Sandbox Stream</div>
            <span className="logo-subtext">
              Safe Sandbox Streaming
            </span>
          </div>
        </div>

        <div className="header-actions">
          <button
            className={`status-indicator-badge ${backendStatus}`}
            onClick={onCheckConnection}
            title={
              backendStatus === 'connected' ? 'Đã kết nối backend. Nhấn để kiểm tra lại.' :
              backendStatus === 'disconnected' ? 'Mất kết nối backend! Nhấn để thử lại.' :
              'Đang kiểm tra kết nối...'
            }
            id="connection-status-button"
          >
            <span className={`status-dot ${backendStatus}`}></span>
            {backendStatus !== 'connected' && (
              <span>
                {backendStatus === 'disconnected' ? 'Backend Offline' : 'Checking...'}
              </span>
            )}
          </button>

          <button
            className="btn-icon"
            onClick={onRefresh}
            disabled={loading}
            title="Làm mới danh sách"
            id="refresh-button"
          >
            <RefreshCw size={16} className={loading ? 'spin' : ''} />
          </button>
          {hasToken && (
            <button
              className="btn-icon delete"
              onClick={onLogout}
              title="Đăng xuất"
              id="logout-button"
            >
              <LogOut size={16} />
            </button>
          )}
        </div>
      </div>

      <nav className="header-nav">
        <button
          className={`nav-tab ${currentView === 'library' || currentView === 'player' ? 'active' : ''}`}
          onClick={() => onViewChange('library')}
          title="Thư viện video"
          id="tab-library"
        >
          <Film size={16} />
          <span>Thư viện</span>
        </button>
        <button
          className={`nav-tab ${currentView === 'upload' ? 'active' : ''}`}
          onClick={() => onViewChange('upload')}
          title="Tải video lên hoặc tải từ liên kết"
          id="tab-upload"
        >
          <UploadCloud size={16} />
          <span>Tải lên / Tải về</span>
        </button>
      </nav>
    </header>
  );
};
