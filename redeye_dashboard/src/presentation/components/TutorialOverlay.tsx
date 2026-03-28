// TutorialOverlay — Interactive Product Tour using driver.js
// Triggers on first dashboard visit (localStorage: 'redeye_hasSeenTutorial').
// Styled to match the Cool Revival Slate/Cyan theme via injected CSS.

import { useEffect } from 'react';
import { driver } from 'driver.js';
import 'driver.js/dist/driver.css';

const TOUR_KEY = 'redeye_hasSeenTutorial';

/** Injects a <style> tag once to override driver.js default colours with our Cyan/Slate palette. */
function injectTourStyles() {
  if (document.getElementById('redeye-tour-styles')) return;
  const style = document.createElement('style');
  style.id = 'redeye-tour-styles';
  style.textContent = `
    /* ── RedEye Tour Theme ────────────────────────────────── */
    .driver-popover {
      background: rgba(15, 23, 42, 0.95) !important;
      border: 1px solid rgba(34, 211, 238, 0.25) !important;
      border-radius: 1rem !important;
      box-shadow: 0 0 40px rgba(34, 211, 238, 0.15), 0 4px 30px rgba(0, 0, 0, 0.6) !important;
      backdrop-filter: blur(20px) !important;
      color: #e2e8f0 !important;
      font-family: inherit !important;
      max-width: 320px !important;
    }
    .driver-popover-title {
      color: #22d3ee !important;
      font-size: 0.875rem !important;
      font-weight: 700 !important;
      letter-spacing: 0.01em !important;
      margin-bottom: 0.4rem !important;
    }
    .driver-popover-description {
      color: #94a3b8 !important;
      font-size: 0.8125rem !important;
      line-height: 1.6 !important;
    }
    .driver-popover-progress-text {
      color: #64748b !important;
      font-size: 0.7rem !important;
    }
    .driver-popover-footer {
      margin-top: 1rem !important;
      gap: 0.5rem !important;
    }
    .driver-popover-prev-btn,
    .driver-popover-next-btn,
    .driver-popover-close-btn {
      background: linear-gradient(135deg, #06b6d4, #2dd4bf) !important;
      color: #020617 !important;
      border: none !important;
      border-radius: 0.5rem !important;
      font-weight: 600 !important;
      font-size: 0.75rem !important;
      padding: 0.4rem 0.85rem !important;
      cursor: pointer !important;
      transition: opacity 0.2s !important;
    }
    .driver-popover-prev-btn {
      background: transparent !important;
      border: 1px solid rgba(100, 116, 139, 0.5) !important;
      color: #94a3b8 !important;
    }
    .driver-popover-prev-btn:hover { opacity: 0.8 !important; }
    .driver-popover-next-btn:hover { opacity: 0.85 !important; }
    .driver-overlay { background: rgba(2, 6, 23, 0.75) !important; }
    .driver-popover-arrow { display: none !important; }
    .driver-active-element {
      outline: 2px solid rgba(34, 211, 238, 0.6) !important;
      outline-offset: 4px !important;
      box-shadow: 0 0 20px rgba(34, 211, 238, 0.2) !important;
      border-radius: 0.75rem !important;
    }
  `;
  document.head.appendChild(style);
}

export function TutorialOverlay() {
  useEffect(() => {
    if (localStorage.getItem(TOUR_KEY)) return;

    injectTourStyles();

    const driverObj = driver({
      showProgress: true,
      animate: true,
      smoothScroll: true,
      overlayColor: 'rgba(2, 6, 23, 0.75)',
      steps: [
        {
          element: '#tour-sidebar',
          popover: {
            title: '🧭 Navigation',
            description: 'Navigate through your gateway settings — metrics, API keys, security alerts, traces, and compliance reports.',
            side: 'right',
            align: 'start',
          },
        },
        {
          element: '#tour-stat-cards',
          popover: {
            title: '📊 Live Metrics',
            description: 'View your real-time LLM token consumption, average latency, threats blocked, and estimated cost. Refreshes every 3 seconds.',
            side: 'bottom',
            align: 'start',
          },
        },
        {
          element: '#tour-traffic-chart',
          popover: {
            title: '📈 Traffic Overview',
            description: 'This chart shows live request volume over time, streamed from your ClickHouse telemetry backend.',
            side: 'top',
            align: 'center',
          },
        },
        {
          element: '#tour-api-keys',
          popover: {
            title: '🔑 API Keys',
            description: 'Manage your secure RedEye routing keys here. Each key is AES-256 encrypted at rest and is the sole credential for your AI gateway.',
            side: 'left',
            align: 'center',
          },
        },
      ],
      onDestroyStarted: () => {
        localStorage.setItem(TOUR_KEY, '1');
        driverObj.destroy();
      },
    });

    // Slight delay to let the dashboard render fully before highlighting
    const timer = setTimeout(() => driverObj.drive(), 800);
    return () => clearTimeout(timer);
  }, []);

  return null; // Renders nothing — driver.js works via DOM
}
