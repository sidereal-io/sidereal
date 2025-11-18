import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import App from './App';

describe('App', () => {
  it('should render hello world message', () => {
    render(<App />);
    expect(screen.getByText('Hello World!')).toBeDefined();
  });

  it('should render Sidereal Frontend title', () => {
    render(<App />);
    expect(screen.getByText('Sidereal Frontend')).toBeDefined();
  });
});
