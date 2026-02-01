// Component that handles server events and shows toast notifications

import { useServerEvents } from '../hooks/useServerEvents';
import { useToast } from './Toast';

export function ServerEventsHandler() {
  const { addToast } = useToast();

  useServerEvents({
    onBookUploaded: ({ title }) => {
      addToast('success', `Book "${title}" was uploaded`);
    },
    onConversionComplete: ({ format }) => {
      addToast('info', `Conversion to ${format.toUpperCase()} complete`);
    },
    onError: ({ message }) => {
      addToast('error', message);
    },
  });

  // This component doesn't render anything
  return null;
}
