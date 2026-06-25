import { apiClient } from './client';
import { ListVideosResponse, UploadUrlResponse, StreamUrlResponse, ActiveDownload } from '../types';

export const videoApi = {
  /**
   * Fetch the list of uploaded videos and storage quotas (supports cache bypass).
   */
  async fetchVideos(forceRefresh?: boolean): Promise<ListVideosResponse> {
    const url = forceRefresh ? '/api/videos?refresh=true' : '/api/videos';
    const response = await apiClient.get<ListVideosResponse>(url);
    return response.data;
  },

  /**
   * Request a Cloudflare R2 Presigned Upload URL.
   */
  async getUploadUrl(fileName: string, fileSize: number): Promise<UploadUrlResponse> {
    const response = await apiClient.post<UploadUrlResponse>('/api/videos/upload-url', {
      file_name: fileName,
      file_size: fileSize,
    });
    return response.data;
  },

  /**
   * Request backend to download a video from a direct URL asynchronously.
   */
  async uploadFromUrl(url: string): Promise<void> {
    await apiClient.post('/api/videos/upload-from-url', { url });
  },

  /**
   * Delete a video from R2.
   */
  async deleteVideo(key: string): Promise<void> {
    await apiClient.delete(`/api/videos/${encodeURIComponent(key)}`);
  },

  /**
   * Rename a video in R2 metadata.
   */
  async renameVideo(key: string, newName: string): Promise<void> {
    await apiClient.post(
      `/api/videos/${encodeURIComponent(key)}/rename`,
      { new_name: newName }
    );
  },

  /**
   * Fetch the direct signed stream URL for a video.
   */
  async getStreamUrl(key: string): Promise<StreamUrlResponse> {
    const response = await apiClient.get<StreamUrlResponse>(
      `/api/videos/${encodeURIComponent(key)}/stream-url`
    );
    return response.data;
  },

  /**
   * Get list of background downloads currently running.
   */
  async getDownloads(): Promise<ActiveDownload[]> {
    const response = await apiClient.get<ActiveDownload[]>('/api/videos/downloads');
    return response.data;
  },

  /**
   * Dismiss/Delete a background download task (typically a failed one).
   */
  async deleteDownload(id: string): Promise<void> {
    await apiClient.delete(`/api/videos/downloads/${id}`);
  },
};
