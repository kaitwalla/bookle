import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { Link } from 'react-router-dom';
import { api } from '../lib/api';
import { FileUpload } from '../components/FileUpload';
import type { BookSummary, UploadResponse } from '../types';

export function Library() {
  const [search, setSearch] = useState('');
  const [showUpload, setShowUpload] = useState(false);
  const [uploadSuccess, setUploadSuccess] = useState<string | null>(null);

  const { data, isLoading, error } = useQuery({
    queryKey: ['books', search],
    queryFn: () => api.listBooks(1, 20, search || undefined),
  });

  const handleUploadSuccess = (response: UploadResponse) => {
    setUploadSuccess(`Successfully uploaded "${response.title}"`);
    setShowUpload(false);
    setTimeout(() => setUploadSuccess(null), 5000);
  };

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex justify-between items-center">
        <h1 className="text-2xl font-bold text-gray-900 dark:text-white">
          Library
        </h1>
        <button
          onClick={() => setShowUpload(!showUpload)}
          className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700"
        >
          {showUpload ? 'Cancel' : 'Upload Book'}
        </button>
      </div>

      {/* Upload section */}
      {showUpload && (
        <FileUpload
          onSuccess={handleUploadSuccess}
          onError={(err) => console.error('Upload error:', err)}
        />
      )}

      {/* Success message */}
      {uploadSuccess && (
        <div className="p-4 bg-green-50 dark:bg-green-900/20 text-green-600 dark:text-green-400 rounded-lg flex justify-between items-center">
          <span>{uploadSuccess}</span>
          <button
            onClick={() => setUploadSuccess(null)}
            className="text-green-700 dark:text-green-300 hover:text-green-900 dark:hover:text-green-100"
          >
            &times;
          </button>
        </div>
      )}

      {/* Search */}
      <div className="flex gap-4">
        <input
          type="text"
          placeholder="Search books..."
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          className="flex-1 px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-800 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500 focus:border-transparent"
        />
      </div>

      {/* Error state */}
      {error && (
        <div className="p-4 bg-red-50 dark:bg-red-900/20 text-red-600 dark:text-red-400 rounded-lg">
          Failed to load books: {error.message}
        </div>
      )}

      {/* Loading state */}
      {isLoading && (
        <div className="flex justify-center py-12">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
        </div>
      )}

      {/* Empty state */}
      {data?.books.length === 0 && !isLoading && (
        <div className="text-center py-12">
          <p className="text-gray-500 dark:text-gray-400 mb-4">
            No books in your library yet.
          </p>
          <button
            onClick={() => setShowUpload(true)}
            className="text-blue-600 dark:text-blue-400 hover:underline"
          >
            Upload your first book
          </button>
        </div>
      )}

      {/* Book grid */}
      {data && data.books.length > 0 && (
        <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-6">
          {data.books.map((book) => (
            <BookCard key={book.id} book={book} />
          ))}
        </div>
      )}

      {/* Pagination info */}
      {data && (
        <div className="text-center text-gray-500 dark:text-gray-400 text-sm">
          Showing {data.books.length} of {data.total} books
        </div>
      )}
    </div>
  );
}

function BookCard({ book }: { book: BookSummary }) {
  return (
    <Link
      to={`/book/${book.id}`}
      className="block bg-white dark:bg-gray-800 rounded-lg shadow hover:shadow-lg transition-shadow overflow-hidden"
    >
      {/* Placeholder cover */}
      <div className="aspect-[2/3] bg-gradient-to-br from-blue-500 to-purple-600 flex items-center justify-center">
        <span className="text-white text-4xl font-bold">
          {book.title.charAt(0).toUpperCase()}
        </span>
      </div>

      {/* Book info */}
      <div className="p-4">
        <h3 className="font-semibold text-gray-900 dark:text-white truncate">
          {book.title}
        </h3>
        {book.authors.length > 0 && (
          <p className="text-sm text-gray-500 dark:text-gray-400 truncate">
            {book.authors.join(', ')}
          </p>
        )}
        <p className="text-xs text-gray-400 dark:text-gray-500 mt-1">
          {book.language.toUpperCase()}
        </p>
      </div>
    </Link>
  );
}
