import { useState, useRef, useCallback } from 'react';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { platform } from '../lib/platform';
import type { UploadResponse } from '../types';

interface FileUploadProps {
  onSuccess?: (response: UploadResponse) => void;
  onError?: (error: Error) => void;
}

export function FileUpload({ onSuccess, onError }: FileUploadProps) {
  const [isDragging, setIsDragging] = useState(false);
  const [uploadProgress, setUploadProgress] = useState<number | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const queryClient = useQueryClient();

  const uploadMutation = useMutation({
    mutationFn: async (file: File) => {
      const formData = new FormData();
      formData.append('file', file);

      return new Promise<UploadResponse>((resolve, reject) => {
        const xhr = new XMLHttpRequest();

        xhr.upload.addEventListener('progress', (event) => {
          if (event.lengthComputable) {
            const percent = Math.round((event.loaded / event.total) * 100);
            setUploadProgress(percent);
          }
        });

        xhr.addEventListener('load', () => {
          if (xhr.status >= 200 && xhr.status < 300) {
            try {
              const response = JSON.parse(xhr.responseText);
              resolve(response);
            } catch {
              reject(new Error('Invalid response from server'));
            }
          } else {
            reject(new Error(`Upload failed: ${xhr.status} ${xhr.statusText}`));
          }
        });

        xhr.addEventListener('error', () => {
          reject(new Error('Network error during upload'));
        });

        xhr.open('POST', `${platform.getApiBaseUrl()}/api/v1/library`);
        xhr.send(formData);
      });
    },
    onSuccess: (response) => {
      setUploadProgress(null);
      queryClient.invalidateQueries({ queryKey: ['books'] });
      onSuccess?.(response);
    },
    onError: (error: Error) => {
      setUploadProgress(null);
      onError?.(error);
    },
  });

  const handleFiles = useCallback((files: FileList | null) => {
    if (!files || files.length === 0) return;

    const file = files[0];
    const validExtensions = ['.epub', '.mobi'];
    const extension = file.name.toLowerCase().slice(file.name.lastIndexOf('.'));

    if (!validExtensions.includes(extension)) {
      onError?.(new Error('Please upload an EPUB or MOBI file'));
      return;
    }

    uploadMutation.mutate(file);
  }, [uploadMutation, onError]);

  const handleDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragging(true);
  }, []);

  const handleDragLeave = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragging(false);
  }, []);

  const handleDrop = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragging(false);
    handleFiles(e.dataTransfer.files);
  }, [handleFiles]);

  const handleClick = useCallback(async () => {
    if (platform.isDesktop()) {
      // Use native file dialog on desktop
      const file = await platform.openFile();
      if (file) {
        uploadMutation.mutate(file);
      }
    } else {
      // Use browser file input
      fileInputRef.current?.click();
    }
  }, [uploadMutation]);

  const handleInputChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    handleFiles(e.target.files);
  }, [handleFiles]);

  const isUploading = uploadMutation.isPending || uploadProgress !== null;

  return (
    <div
      className={`
        relative border-2 border-dashed rounded-lg p-8 text-center transition-colors cursor-pointer
        ${isDragging
          ? 'border-blue-500 bg-blue-50 dark:bg-blue-900/20'
          : 'border-gray-300 dark:border-gray-600 hover:border-gray-400 dark:hover:border-gray-500'
        }
        ${isUploading ? 'pointer-events-none opacity-75' : ''}
      `}
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
      onClick={handleClick}
    >
      <input
        ref={fileInputRef}
        type="file"
        accept=".epub,.mobi"
        onChange={handleInputChange}
        className="hidden"
      />

      {isUploading ? (
        <div className="space-y-4">
          <div className="w-16 h-16 mx-auto">
            <svg className="animate-spin w-full h-full text-blue-600" fill="none" viewBox="0 0 24 24">
              <circle
                className="opacity-25"
                cx="12"
                cy="12"
                r="10"
                stroke="currentColor"
                strokeWidth="4"
              />
              <path
                className="opacity-75"
                fill="currentColor"
                d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
              />
            </svg>
          </div>

          {uploadProgress !== null && (
            <div className="space-y-2">
              <div className="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-2.5">
                <div
                  className="bg-blue-600 h-2.5 rounded-full transition-all duration-300"
                  style={{ width: `${uploadProgress}%` }}
                />
              </div>
              <p className="text-sm text-gray-600 dark:text-gray-400">
                Uploading... {uploadProgress}%
              </p>
            </div>
          )}
        </div>
      ) : (
        <div className="space-y-4">
          <div className="w-16 h-16 mx-auto text-gray-400 dark:text-gray-500">
            <svg fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={1.5}
                d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"
              />
            </svg>
          </div>

          <div>
            <p className="text-gray-900 dark:text-white font-medium">
              {isDragging ? 'Drop your book here' : 'Drag and drop your book here'}
            </p>
            <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
              or click to browse
            </p>
          </div>

          <p className="text-xs text-gray-400 dark:text-gray-500">
            Supported formats: EPUB, MOBI
          </p>
        </div>
      )}

      {uploadMutation.error && (
        <div className="mt-4 p-3 bg-red-50 dark:bg-red-900/20 text-red-600 dark:text-red-400 rounded-lg text-sm">
          {uploadMutation.error.message}
        </div>
      )}
    </div>
  );
}
