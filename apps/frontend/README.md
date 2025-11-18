# Sidereal Frontend

React-based frontend application built with Vite, Tailwind CSS, and TypeScript.

## Technology Stack

- **React 19**: UI framework
- **Vite**: Build tool and development server
- **Tailwind CSS**: Utility-first CSS framework
- **TypeScript**: Type-safe JavaScript
- **Vitest**: Unit testing framework
- **Playwright**: End-to-end testing framework

## Development

### Install Dependencies

From the project root:
```bash
pnpm install
```

### Run Development Server

```bash
pnpm dev
```

The app will be available at http://localhost:3000 with hot-reload enabled.

### Available Scripts

- `pnpm dev` - Start development server
- `pnpm build` - Build for production
- `pnpm preview` - Preview production build
- `pnpm test` - Run unit tests
- `pnpm test:ui` - Run tests with UI
- `pnpm test:coverage` - Run tests with coverage report
- `pnpm test:e2e` - Run end-to-end tests
- `pnpm test:e2e:ui` - Run E2E tests with UI
- `pnpm lint` - Lint code
- `pnpm type-check` - Type check without building

## Project Structure

```
apps/frontend/
├── src/
│   ├── App.tsx           # Main application component
│   ├── main.tsx          # Application entry point
│   ├── index.css         # Global styles with Tailwind directives
│   ├── test/             # Test utilities and setup
│   └── App.test.tsx      # Example unit test
├── e2e/                  # End-to-end tests
├── public/               # Static assets
├── index.html            # HTML entry point
├── vite.config.ts        # Vite configuration
├── tailwind.config.js    # Tailwind CSS configuration
├── vitest.config.ts      # Vitest configuration
└── playwright.config.ts  # Playwright configuration
```

## Adding Components

This project uses Tailwind CSS for styling. Create new components in `src/components/`:

```tsx
// src/components/MyComponent.tsx
export function MyComponent() {
  return (
    <div className="p-4 bg-blue-500 text-white">
      Hello from MyComponent
    </div>
  );
}
```

## Environment Variables

Create a `.env` file based on `.env.example`:

```bash
VITE_API_URL=http://localhost:3001
```

Access in code:
```typescript
const apiUrl = import.meta.env.VITE_API_URL;
```

## Docker Development

The frontend runs in a Docker container with volume mounts for hot-reload:

```bash
# From project root
docker compose up frontend
```

## Testing

### Unit Tests

```bash
# Run tests in watch mode
pnpm test

# Run tests with coverage
pnpm test:coverage

# Run tests with UI
pnpm test:ui
```

### E2E Tests

```bash
# Run E2E tests
pnpm test:e2e

# Run E2E tests with UI
pnpm test:e2e:ui
```
