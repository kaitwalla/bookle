// Hook for subscribing to server-sent events and updating UI

import { useEffect, useCallback, useRef } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { api } from '../lib/api';

export interface ServerEvent {
  type: 'book_uploaded' | 'conversion_complete' | 'error';
  data: {
    id?: string;
    title?: string;
    format?: string;
    message?: string;
  };
}

interface UseServerEventsOptions {
  onBookUploaded?: (data: { id: string; title: string }) => void;
  onConversionComplete?: (data: { id: string; format: string }) => void;
  onError?: (data: { message: string }) => void;
}

export function useServerEvents(options: UseServerEventsOptions = {}) {
  const queryClient = useQueryClient();
  const optionsRef = useRef(options);
  optionsRef.current = options;

  const handleEvent = useCallback(
    (event: { type: string; data: unknown }) => {
      const serverEvent = event as ServerEvent;

      switch (serverEvent.type) {
        case 'book_uploaded':
          // Invalidate the books list to refetch
          queryClient.invalidateQueries({ queryKey: ['books'] });

          if (optionsRef.current.onBookUploaded && serverEvent.data.id && serverEvent.data.title) {
            optionsRef.current.onBookUploaded({
              id: serverEvent.data.id,
              title: serverEvent.data.title,
            });
          }
          break;

        case 'conversion_complete':
          // Invalidate the specific book query
          if (serverEvent.data.id) {
            queryClient.invalidateQueries({ queryKey: ['book', serverEvent.data.id] });
          }

          if (optionsRef.current.onConversionComplete && serverEvent.data.id && serverEvent.data.format) {
            optionsRef.current.onConversionComplete({
              id: serverEvent.data.id,
              format: serverEvent.data.format,
            });
          }
          break;

        case 'error':
          if (optionsRef.current.onError && serverEvent.data.message) {
            optionsRef.current.onError({
              message: serverEvent.data.message,
            });
          }
          break;
      }
    },
    [queryClient]
  );

  useEffect(() => {
    // Subscribe to SSE events
    const cleanup = api.subscribeToEvents(handleEvent);

    // Cleanup on unmount
    return cleanup;
  }, [handleEvent]);
}
