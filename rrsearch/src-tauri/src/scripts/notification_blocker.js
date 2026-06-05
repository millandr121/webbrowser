(function () {
  'use strict';

  // ── Block notification permission requests ───────────────────────────────
  // Override the API before any page script can touch it
  Object.defineProperty(Notification, 'permission', {
    get: () => 'denied',
    configurable: false,
  });

  const _original = Notification.requestPermission.bind(Notification);
  Notification.requestPermission = function () {
    // silently return denied — no popup, no prompt, nothing
    return Promise.resolve('denied');
  };

  // ── Block push permission via Permissions API ────────────────────────────
  if (navigator.permissions && navigator.permissions.query) {
    const _query = navigator.permissions.query.bind(navigator.permissions);
    navigator.permissions.query = function (desc) {
      if (desc && (desc.name === 'notifications' || desc.name === 'push')) {
        return Promise.resolve({ state: 'denied', onchange: null });
      }
      return _query(desc);
    };
  }

  // ── Block ServiceWorker push subscriptions ───────────────────────────────
  if (navigator.serviceWorker) {
    const _getRegistration = navigator.serviceWorker.getRegistration.bind(navigator.serviceWorker);
    navigator.serviceWorker.getRegistration = async function (...args) {
      const reg = await _getRegistration(...args);
      if (reg && reg.pushManager) {
        reg.pushManager.subscribe = () => Promise.reject(new Error('rrsearch: push blocked'));
        reg.pushManager.permissionState = () => Promise.resolve('denied');
      }
      return reg;
    };
  }

  // ── Hide "allow notifications" UI banners sites inject themselves ────────
  const NOTI_SELECTORS = [
    '[class*="notification-prompt"]',
    '[class*="push-prompt"]',
    '[class*="subscribe-prompt"]',
    '[id*="notification-prompt"]',
    '[id*="push-prompt"]',
    '[class*="push-notification-bar"]',
    '[class*="allow-notifications"]',
  ].join(',');

  function killNotiUI() {
    document.querySelectorAll(NOTI_SELECTORS).forEach(el => {
      el.style.cssText = 'display:none!important';
    });
  }

  killNotiUI();
  const obs = new MutationObserver(killNotiUI);
  obs.observe(document.documentElement, { childList: true, subtree: true });
  setTimeout(() => obs.disconnect(), 15000);
})();
