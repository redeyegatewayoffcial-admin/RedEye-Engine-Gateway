// Presentation Hook — useTheme
// Toggles the `.dark` class on <html> for Spatial UI.
// Light mode is the default `:root` state — CSS variables defined under `:root`
// use the inverted Obsidian palette (#FFFFFF canvas, #F5F5F5 surface).
// Dark mode applies `.dark` to activate the full Obsidian Command palette.
// Triggers smooth CSS background/color transitions defined in index.css.

import { useState, useEffect, useCallback } from 'react';

type Theme = 'dark' | 'light';

const STORAGE_KEY = 're_theme';

function getInitialTheme(): Theme {
  if (typeof window === 'undefined') return 'dark';
  const stored = localStorage.getItem(STORAGE_KEY) as Theme | null;
  if (stored === 'dark' || stored === 'light') return stored;
  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
}

/**
 * Applies the theme to the HTML root element.
 *
 * Strategy (matches CSS architecture in index.css):
 *   - Dark  → add `.dark`  to <html>; colorScheme = 'dark'
 *   - Light → remove `.dark` from <html>; colorScheme = 'light'
 *             `:root` already holds the light-mode inverted palette
 *             (--bg-canvas: #FFFFFF, --surface: #F5F5F5). No `.light`
 *             class is needed — light is the absence of `.dark`.
 */
function applyTheme(theme: Theme) {
  const root = document.documentElement;
  if (theme === 'dark') {
    root.classList.add('dark');
    root.style.colorScheme = 'dark';
  } else {
    root.classList.remove('dark');
    root.style.colorScheme = 'light';
  }
}

export function useTheme() {
  const [theme, setTheme] = useState<Theme>(getInitialTheme);

  useEffect(() => {
    applyTheme(theme);
    localStorage.setItem(STORAGE_KEY, theme);
  }, [theme]);

  const toggleTheme = useCallback(() => {
    setTheme((t) => (t === 'dark' ? 'light' : 'dark'));
  }, []);

  return { theme, toggleTheme } as const;
}
