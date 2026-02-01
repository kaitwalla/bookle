// API client for Bookle server

import { platform } from './platform';
import type { BookResponse, ListBooksResponse, UploadResponse } from '../types';

const BASE_URL = platform.getApiBaseUrl();

async function fetchApi<T>(path: string, options?: RequestInit): Promise<T> {
  const response = await fetch(`${BASE_URL}${path}`, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options?.headers,
    },
  });

  if (!response.ok) {
    throw new Error(`API error: ${response.status} ${response.statusText}`);
  }

  return response.json();
}

// Library API
export const api = {
  // List books with pagination
  async listBooks(page = 1, perPage = 20, search?: string): Promise<ListBooksResponse> {
    const params = new URLSearchParams({
      page: page.toString(),
      per_page: perPage.toString(),
    });
    if (search) {
      params.set('search', search);
    }
    return fetchApi(`/api/v1/library?${params}`);
  },

  // Get single book
  async getBook(id: string): Promise<BookResponse> {
    return fetchApi(`/api/v1/library/${id}`);
  },

  // Upload a book
  async uploadBook(file: File): Promise<UploadResponse> {
    const formData = new FormData();
    formData.append('file', file);

    const response = await fetch(`${BASE_URL}/api/v1/library`, {
      method: 'POST',
      body: formData,
    });

    if (!response.ok) {
      throw new Error(`Upload failed: ${response.status}`);
    }

    return response.json();
  },

  // Delete a book
  async deleteBook(id: string): Promise<void> {
    await fetch(`${BASE_URL}/api/v1/library/${id}`, {
      method: 'DELETE',
    });
  },

  // Download a book in a specific format
  async downloadBook(id: string, format: 'epub' | 'pdf'): Promise<Blob> {
    const response = await fetch(`${BASE_URL}/api/v1/library/${id}/download?format=${format}`);
    if (!response.ok) {
      throw new Error(`Download failed: ${response.status}`);
    }
    return response.blob();
  },

  // Subscribe to SSE events
  subscribeToEvents(onEvent: (event: { type: string; data: unknown }) => void): () => void {
    const eventSource = new EventSource(`${BASE_URL}/api/v1/sync`);

    eventSource.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        onEvent({ type: event.type, data });
      } catch {
        console.error('Failed to parse SSE event:', event);
      }
    };

    eventSource.onerror = () => {
      console.error('SSE connection error');
    };

    // Return cleanup function
    return () => eventSource.close();
  },
};
