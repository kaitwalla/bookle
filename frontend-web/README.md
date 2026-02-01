# frontend-web

React-based web interface for the Bookle ebook management system.

## Features

- Book library with search and pagination
- Drag-and-drop file upload
- Book detail view with chapter list
- Download in multiple formats (EPUB, PDF)
- Real-time updates via Server-Sent Events
- Dark mode support
- Responsive design

## Tech Stack

- **React 18** with TypeScript
- **React Router v7** for routing
- **TanStack Query (React Query)** for data fetching
- **Tailwind CSS v4** for styling
- **Vite** for build tooling

## Development

### Prerequisites

- Node.js 18+
- npm or pnpm
- Running bookle-server (default: http://localhost:3000)

### Setup

```bash
cd frontend-web
npm install
```

### Running

```bash
# Development server (http://localhost:5173)
npm run dev

# Build for production
npm run build

# Preview production build
npm run preview
```

## Project Structure

```
frontend-web/
├── src/
│   ├── components/
│   │   ├── FileUpload.tsx    # Drag-drop upload
│   │   ├── Layout.tsx        # App shell
│   │   ├── Toast.tsx         # Notifications
│   │   └── ServerEventsHandler.tsx
│   ├── pages/
│   │   ├── Library.tsx       # Book grid
│   │   └── BookDetail.tsx    # Book view
│   ├── hooks/
│   │   └── useServerEvents.ts
│   ├── lib/
│   │   ├── api.ts            # API client
│   │   └── platform.ts       # Web/Tauri detection
│   ├── types/
│   │   └── index.ts          # TypeScript types
│   ├── App.tsx               # Root component
│   ├── main.tsx              # Entry point
│   └── index.css             # Global styles
├── public/
├── index.html
├── package.json
├── tsconfig.json
├── vite.config.ts
└── tailwind.config.js
```

## Components

### Layout

App shell with header and navigation.

### Library

Book grid with:
- Search filtering
- Upload modal
- Pagination
- Empty state

### BookDetail

Book information view with:
- Metadata display
- Chapter list
- Download buttons
- Delete action

### FileUpload

Drag-and-drop file upload with:
- Progress indicator
- Format validation
- Error handling

### Toast

Toast notification system for:
- Success messages
- Error alerts
- Real-time event notifications

## Hooks

### useServerEvents

Subscribes to SSE and invalidates React Query cache:

```typescript
import { useServerEvents } from '../hooks/useServerEvents';

useServerEvents({
  onBookUploaded: ({ id, title }) => {
    console.log(`Book uploaded: ${title}`);
  },
  onConversionComplete: ({ id, format }) => {
    console.log(`Conversion complete: ${format}`);
  },
  onError: ({ message }) => {
    console.error(message);
  },
});
```

## API Client

```typescript
import { api } from '../lib/api';

// List books
const { books, total } = await api.listBooks(1, 20, 'search');

// Get book
const book = await api.getBook('uuid');

// Upload book
const result = await api.uploadBook(file);

// Delete book
await api.deleteBook('uuid');

// Download book
const blob = await api.downloadBook('uuid', 'epub');

// Subscribe to events
const unsubscribe = api.subscribeToEvents((event) => {
  console.log(event);
});
```

## Configuration

### API Base URL

Set via `platform.ts`:

```typescript
// Web: Uses relative URL or environment variable
// Tauri: Uses localhost:3000 or embedded server
```

### Environment Variables

Create `.env.local`:

```
VITE_API_URL=http://localhost:3000
```

## Building for Production

```bash
npm run build
```

Output in `dist/` directory. Serve with any static file server:

```bash
npx serve dist
```

## Styling

Uses Tailwind CSS v4 with:
- Dark mode (`dark:` variants)
- Responsive design (`sm:`, `md:`, `lg:`)
- Custom animations (toast slide-in)

Custom CSS in `src/index.css`:

```css
@import "tailwindcss";

:root {
  --color-primary: #3b82f6;
}

.animate-slide-in {
  animation: slide-in 0.3s ease-out;
}
```

## TypeScript Types

```typescript
interface BookSummary {
  id: string;
  title: string;
  authors: string[];
  language: string;
}

interface BookResponse {
  id: string;
  title: string;
  authors: string[];
  description?: string;
  language: string;
  chapters: ChapterSummary[];
}
```

## License

MIT
