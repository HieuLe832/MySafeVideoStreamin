/**
 * Validates a file's content signature (magic bytes) to verify it is a real video.
 * Helps prevent users from uploading malicious files renamed with video extensions.
 */
export const isVideoFileSignature = async (file: File): Promise<boolean> => {
  return new Promise((resolve) => {
    const reader = new FileReader();
    reader.onloadend = (e) => {
      if (!e.target || !e.target.result) {
        resolve(false);
        return;
      }
      const arr = new Uint8Array(e.target.result as ArrayBuffer);
      if (arr.length < 12) {
        resolve(false);
        return;
      }

      const toHex = (start: number, end: number) => {
        return Array.from(arr.slice(start, end))
          .map((b) => b.toString(16).padStart(2, '0').toUpperCase())
          .join(' ');
      };

      const toAscii = (start: number, end: number) => {
        return String.fromCharCode(...arr.slice(start, end));
      };

      const hexStart4 = toHex(0, 4);
      const ftypText = toAscii(4, 8);
      const riffText = toAscii(0, 4);
      const aviText = toAscii(8, 12);

      // 1. MKV/WebM (EBML: 1A 45 DF A3)
      if (hexStart4 === '1A 45 DF A3') {
        resolve(true);
        return;
      }

      // 2. MP4/MOV (ftyp at offset 4)
      if (ftypText === 'ftyp') {
        resolve(true);
        return;
      }

      // 3. AVI (RIFF + AVI )
      if (riffText === 'RIFF' && aviText === 'AVI ') {
        resolve(true);
        return;
      }

      // 4. MPEG (00 00 01 BA or 00 00 01 B3)
      const mpegHex = toHex(0, 4);
      if (mpegHex === '00 00 01 BA' || mpegHex === '00 00 01 B3') {
        resolve(true);
        return;
      }

      resolve(false);
    };

    // Read the first 16 bytes of the file to verify the signature
    const blob = file.slice(0, 16);
    reader.readAsArrayBuffer(blob);
  });
};
