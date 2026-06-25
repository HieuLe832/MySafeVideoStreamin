/**
 * Formats a number of bytes into a human-readable string (e.g., KB, MB, GB).
 */
export const formatBytes = (bytes: number, decimals = 2): string => {
  if (bytes === 0) return '0 Bytes';
  const k = 1024;
  const dm = decimals < 0 ? 0 : decimals;
  const sizes = ['Bytes', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(dm)) + ' ' + sizes[i];
};

/**
 * Removes the .mp4 extension (or other common video extensions) from a filename.
 */
export const cleanVideoName = (filename: string): string => {
  return filename.replace(/\.(mp4|mkv|avi|mov|flv|wmv)$/i, '');
};
