// Runtime config served by the backend at GET /config.js. The script tag in
// index.html runs synchronously before the main module, so window.__APP_CONFIG
// is always defined by the time any React code reads it (the module script
// is deferred by default, so it executes after all classic scripts).
//
// If /config.js fails to load (e.g. backend down), window.__APP_CONFIG is
// undefined and we fall back to "no Google client" → password mode UI.

interface AppConfig {
  googleClientId?: string;
}

declare global {
  interface Window {
    __APP_CONFIG?: AppConfig;
  }
}

export function getGoogleClientId(): string {
  return window.__APP_CONFIG?.googleClientId ?? '';
}

export function isGoogleAuthEnabled(): boolean {
  return Boolean(getGoogleClientId());
}
