import { apiClient } from './client';

export interface GoogleConfigResponse {
  google_client_id: string;
}

export const authApi = {
  /**
   * Fetch the Google Sign-In Client ID from backend configuration.
   */
  async getGoogleConfig(): Promise<GoogleConfigResponse> {
    const response = await apiClient.get<GoogleConfigResponse>('/api/auth/config');
    return response.data;
  },

  /**
   * Check connection status of the backend.
   */
  async checkHealth(): Promise<void> {
    await apiClient.get('/health', { timeout: 5000 });
  },
};
