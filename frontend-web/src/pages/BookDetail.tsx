import { useParams, useNavigate } from 'react-router-dom';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { api } from '../lib/api';
import { platform } from '../lib/platform';

export function BookDetail() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const queryClient = useQueryClient();

  const { data: book, isLoading, error } = useQuery({
    queryKey: ['book', id],
    queryFn: () => api.getBook(id!),
    enabled: !!id,
  });

  const deleteMutation = useMutation({
    mutationFn: () => api.deleteBook(id!),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['books'] });
      navigate('/');
    },
  });

  const handleDownload = async (format: 'epub' | 'pdf') => {
    if (!id || !book) return;

    try {
      const blob = await api.downloadBook(id, format);
      const filename = `${book.title}.${format}`;
      await platform.saveFile(blob, filename);
    } catch (err) {
      console.error('Download failed:', err);
    }
  };

  const handleDelete = () => {
    if (confirm('Are you sure you want to delete this book?')) {
      deleteMutation.mutate();
    }
  };

  if (isLoading) {
    return (
      <div className="flex justify-center py-12">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
      </div>
    );
  }

  if (error || !book) {
    return (
      <div className="text-center py-12">
        <p className="text-red-600 dark:text-red-400">
          {error?.message || 'Book not found'}
        </p>
        <button
          onClick={() => navigate('/')}
          className="mt-4 text-blue-600 dark:text-blue-400 hover:underline"
        >
          Back to Library
        </button>
      </div>
    );
  }

  return (
    <div className="max-w-4xl mx-auto">
      {/* Back button */}
      <button
        onClick={() => navigate('/')}
        className="mb-6 text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white flex items-center gap-2"
      >
        <span>&larr;</span> Back to Library
      </button>

      <div className="bg-white dark:bg-gray-800 rounded-lg shadow overflow-hidden">
        {/* Header */}
        <div className="p-6 border-b border-gray-200 dark:border-gray-700">
          <h1 className="text-2xl font-bold text-gray-900 dark:text-white">
            {book.title}
          </h1>
          {book.authors.length > 0 && (
            <p className="text-gray-600 dark:text-gray-400 mt-1">
              by {book.authors.join(', ')}
            </p>
          )}
        </div>

        {/* Details */}
        <div className="p-6 space-y-4">
          <div>
            <h2 className="text-sm font-medium text-gray-500 dark:text-gray-400">
              Language
            </h2>
            <p className="text-gray-900 dark:text-white">
              {book.language.toUpperCase()}
            </p>
          </div>

          {book.description && (
            <div>
              <h2 className="text-sm font-medium text-gray-500 dark:text-gray-400">
                Description
              </h2>
              <p className="text-gray-900 dark:text-white">{book.description}</p>
            </div>
          )}

          <div>
            <h2 className="text-sm font-medium text-gray-500 dark:text-gray-400">
              Chapters ({book.chapters.length})
            </h2>
            <ul className="mt-2 space-y-1">
              {book.chapters.map((chapter, i) => (
                <li
                  key={i}
                  className="text-gray-700 dark:text-gray-300 text-sm"
                >
                  {i + 1}. {chapter.title}
                </li>
              ))}
            </ul>
          </div>
        </div>

        {/* Actions */}
        <div className="p-6 bg-gray-50 dark:bg-gray-900/50 border-t border-gray-200 dark:border-gray-700 flex gap-4">
          <button
            onClick={() => handleDownload('epub')}
            className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700"
          >
            Download EPUB
          </button>
          <button
            onClick={() => handleDownload('pdf')}
            className="px-4 py-2 bg-purple-600 text-white rounded-lg hover:bg-purple-700"
          >
            Download PDF
          </button>
          <button
            onClick={handleDelete}
            disabled={deleteMutation.isPending}
            className="px-4 py-2 bg-red-600 text-white rounded-lg hover:bg-red-700 disabled:opacity-50 ml-auto"
          >
            {deleteMutation.isPending ? 'Deleting...' : 'Delete'}
          </button>
        </div>
      </div>
    </div>
  );
}
