import { useState, useEffect } from 'react';
import { authApi } from '../api/authApi';
import { videoApi } from '../api/videoApi';
import { ListVideosResponse } from '../types';

export const useAuth = () => {
  const [isAuthenticated, setIsAuthenticated] = useState<boolean>(() => {
    return !!localStorage.getItem('access_token');
  });
  const [googleClientId, setGoogleClientId] = useState<string | null>(null);
  const [authError, setAuthError] = useState<string | null>(null);
  const [loading, setLoading] = useState<boolean>(false);

  // Fetch Google Client ID from backend configuration on mount
  useEffect(() => {
    authApi.getGoogleConfig()
      .then((data) => {
        setGoogleClientId(data.google_client_id);
      })
      .catch((err) => {
        console.error("Không thể tải cấu hình Google Auth từ server:", err);
      });
  }, []);

  /**
   * Handle Google Login flow using credential token.
   * Optimistically validates token by executing fetchVideos request.
   */
  const handleGoogleLogin = async (idToken: string, onSuccess: (data: ListVideosResponse) => void) => {
    setLoading(true);
    setAuthError(null);
    try {
      localStorage.setItem('access_token', idToken);
      const data = await videoApi.fetchVideos();
      setIsAuthenticated(true);
      onSuccess(data);
    } catch (err: any) {
      localStorage.removeItem('access_token');
      setIsAuthenticated(false);
      setAuthError(
        err.response?.data?.error || 
        'Xác thực tài khoản Google thất bại hoặc bạn không được cấp quyền truy cập.'
      );
    } finally {
      setLoading(false);
    }
  };

  /**
   * Logs out the user by removing token from localStorage and cleaning state.
   */
  const handleLogout = (onLogoutSuccess: () => void) => {
    localStorage.removeItem('access_token');
    setIsAuthenticated(false);
    setAuthError(null);
    onLogoutSuccess();
  };

  return {
    isAuthenticated,
    setIsAuthenticated,
    googleClientId,
    authError,
    setAuthError,
    loading,
    setLoading,
    handleGoogleLogin,
    handleLogout,
  };
};
