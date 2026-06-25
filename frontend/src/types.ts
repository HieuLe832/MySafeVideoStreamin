export interface Video {
  key: string;
  original_name: string;
  size: number;
  uploaded_at: string;
  stream_url: string;
}

export interface ListVideosResponse {
  videos: Video[];
  total_used_bytes: number;
  max_limit_bytes: number;
}

export interface UploadUrlResponse {
  upload_url: string;
  key: string;
}

export interface StreamUrlResponse {
  stream_url: string;
}

export interface ActiveDownload {
  id: string;
  url: string;
  filename: string;
  status: string; // "Downloading" | "Uploading" | "Failed"
  error?: string;
  downloaded_bytes: number;
  total_bytes?: number;
}

