// import '@testing-library/jest-dom';
// import { cleanup } from '@testing-library/react';
// import { afterEach, vi } from 'vitest';

// // Fix for React 19 Act warnings
// // @ts-ignore
// global.IS_REACT_ACT_ENVIRONMENT = true;

// // Automatically cleanup after each test
// afterEach(() => {
//   cleanup();
// });

// // Mocking browser APIs that JSDOM doesn't support well
// Object.defineProperty(window, 'matchMedia', {
//   writable: true,
//   value: vi.fn().mockImplementation(query => ({
//     matches: false,
//     media: query,
//     onchange: null,
//     addListener: vi.fn(), // Deprecated
//     removeListener: vi.fn(), // Deprecated
//     addEventListener: vi.fn(),
//     removeEventListener: vi.fn(),
//     dispatchEvent: vi.fn(),
//   })),
// });

// global.ResizeObserver = vi.fn().mockImplementation(() => ({
//   observe: vi.fn(),
//   unobserve: vi.fn(),
//   disconnect: vi.fn(),
// }));
