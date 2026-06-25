import React, { useEffect } from 'react';
import { Lock, RefreshCw, ShieldAlert } from 'lucide-react';

declare global {
  interface Window {
    google: any;
  }
}

interface LoginProps {
  googleClientId: string | null;
  loading: boolean;
  authError: string | null;
  onGoogleLogin: (idToken: string) => void;
  backendStatus: 'checking' | 'connected' | 'disconnected';
  onCheckConnection: () => void;
}

export const Login: React.FC<LoginProps> = ({
  googleClientId,
  loading,
  authError,
  onGoogleLogin,
  backendStatus,
  onCheckConnection,
}) => {
  // Initialize Google One Tap and Google Sign-In Button when mounted
  useEffect(() => {
    if (!googleClientId) return;

    const initGoogleSignIn = () => {
      if (window.google?.accounts?.id) {
        window.google.accounts.id.initialize({
          client_id: googleClientId,
          callback: (response: any) => {
            if (response.credential) {
              onGoogleLogin(response.credential);
            }
          },
          auto_select: false,
          cancel_on_tap_outside: true,
        });

        // Render Google Sign-in button
        window.google.accounts.id.renderButton(
          document.getElementById('google-signin-btn'),
          {
            theme: 'filled_blue',
            size: 'large',
            width: 320,
            text: 'signin_with',
            shape: 'pill',
          }
        );

        // Kích hoạt Google One Tap prompt
        window.google.accounts.id.prompt();
      } else {
        // Retry if Google script is not fully loaded
        setTimeout(initGoogleSignIn, 500);
      }
    };

    initGoogleSignIn();
  }, [googleClientId, onGoogleLogin]);

  return (
    <div style={{ display: 'flex', justifyContent: 'center', alignItems: 'center', minHeight: '100vh', padding: '1rem' }}>
      <div className="card" style={{ position: 'relative', maxWidth: '440px', width: '100%', padding: '3.5rem 2.5rem', textAlign: 'center', border: '1px solid var(--border-color)', borderRadius: 'var(--radius-lg)', background: 'var(--bg-glass)', backdropFilter: 'blur(20px)', boxShadow: 'var(--shadow-lg)' }}>
        <div style={{ position: 'absolute', top: '1.25rem', right: '1.25rem' }}>
          <button
            className={`status-indicator-badge ${backendStatus}`}
            onClick={onCheckConnection}
            title={
              backendStatus === 'connected' ? 'Đã kết nối backend. Nhấn để kiểm tra lại.' :
              backendStatus === 'disconnected' ? 'Mất kết nối backend! Nhấn để thử lại.' :
              'Đang kiểm tra kết nối...'
            }
            id="login-connection-status-button"
          >
            <span className={`status-dot ${backendStatus}`}></span>
            {backendStatus !== 'connected' && (
              <span>
                {backendStatus === 'disconnected' ? 'Offline' : 'Checking...'}
              </span>
            )}
          </button>
        </div>
        <div className="logo-icon" style={{ margin: '0 auto 1.5rem auto', width: '64px', height: '64px', borderRadius: '20px', display: 'flex', alignItems: 'center', justifyContent: 'center', background: 'var(--gradient-brand)', boxShadow: 'var(--shadow-glow)' }}>
          <Lock size={32} style={{ color: 'white' }} />
        </div>
        <h1 style={{ fontSize: '2rem', marginBottom: '0.75rem', fontFamily: 'var(--font-title)', fontWeight: 800 }}>Sandbox Stream</h1>
        <p style={{ color: 'var(--text-secondary)', fontSize: '0.9rem', marginBottom: '2.5rem', lineHeight: '1.6' }}>
          Hệ thống truyền tải và stream video trực tuyến. Vui lòng đăng nhập bằng tài khoản Google được cấp quyền để tiếp tục.
        </p>

        <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '1.5rem', width: '100%' }}>
          {/* Google Sign-in Button Container */}
          <div id="google-signin-btn" style={{ minHeight: '44px', display: 'flex', justifyContent: 'center', width: '100%' }}></div>

          {loading && (
            <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', color: 'var(--color-primary)', fontSize: '0.9rem' }}>
              <RefreshCw size={16} className="spin" />
              <span>Đang xác thực thông tin...</span>
            </div>
          )}

          {authError && (
            <div className="alert-box error" style={{ margin: '1rem 0 0 0', width: '100%', textAlign: 'left', fontSize: '0.85rem' }}>
              <ShieldAlert size={16} style={{ flexShrink: 0 }} />
              <div>{authError}</div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
