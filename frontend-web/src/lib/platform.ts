// Platform abstraction for web vs Tauri

export interface PlatformAdapter {
  isDesktop(): boolean;
  openFile(): Promise<File | null>;
  saveFile(data: Blob, filename: string): Promise<void>;
  getApiBaseUrl(): string;
}

// Web platform implementation
class WebPlatformAdapter implements PlatformAdapter {
  isDesktop(): boolean {
    return false;
  }

  async openFile(): Promise<File | null> {
    return new Promise((resolve) => {
      const input = document.createElement('input');
      input.type = 'file';
      input.accept = '.epub,.mobi,.pdf';
      input.onchange = () => {
        const file = input.files?.[0] || null;
        resolve(file);
      };
      input.click();
    });
  }

  async saveFile(data: Blob, filename: string): Promise<void> {
    const url = URL.createObjectURL(data);
    const a = document.createElement('a');
    a.href = url;
    a.download = filename;
    a.click();
    URL.revokeObjectURL(url);
  }

  getApiBaseUrl(): string {
    return import.meta.env.VITE_API_URL || 'http://localhost:3000';
  }
}

// Tauri platform implementation (stub - would use @tauri-apps/api)
class TauriPlatformAdapter implements PlatformAdapter {
  isDesktop(): boolean {
    return true;
  }

  async openFile(): Promise<File | null> {
    // Would use: import { open } from '@tauri-apps/plugin-dialog';
    // const selected = await open({ filters: [{ name: 'Ebooks', extensions: ['epub', 'mobi', 'pdf'] }] });
    console.warn('Tauri file dialog not implemented');
    return null;
  }

  async saveFile(data: Blob, filename: string): Promise<void> {
    // Would use: import { save } from '@tauri-apps/plugin-dialog';
    // const path = await save({ defaultPath: filename });
    console.warn('Tauri save dialog not implemented', data, filename);
  }

  getApiBaseUrl(): string {
    // In Tauri, we'd use IPC instead of HTTP
    return 'tauri://localhost';
  }
}

// Detect platform and export adapter
function detectPlatform(): PlatformAdapter {
  // Check if we're in Tauri
  if (typeof window !== 'undefined' && '__TAURI__' in window) {
    return new TauriPlatformAdapter();
  }
  return new WebPlatformAdapter();
}

export const platform = detectPlatform();
