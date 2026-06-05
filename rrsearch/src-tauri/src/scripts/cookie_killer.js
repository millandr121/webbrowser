(function () {
  'use strict';

  // ── CSS nuke: hide banners before they paint ────────────────────────────
  const STYLE = `
    /* OneTrust */
    #onetrust-banner-sdk, #onetrust-consent-sdk, .onetrust-pc-dark-filter,
    /* Cookiebot */
    #CybotCookiebotDialog, #CybotCookiebotDialogBodyUnderlay,
    /* Quantcast */
    #qc-cmp2-container, #qc-cmp2-ui,
    /* TrustArc */
    #truste-consent-content, .truste_overlay, .truste_box_overlay,
    /* Generic patterns */
    [class*="cookie-banner"], [class*="cookiebanner"], [class*="cookie_banner"],
    [class*="cookie-consent"], [class*="cookieconsent"], [class*="cookie_consent"],
    [class*="cookie-notice"], [class*="cookienotice"],
    [class*="cookie-popup"], [class*="cookie-modal"],
    [id*="cookie-banner"], [id*="cookiebanner"], [id*="cookie_banner"],
    [id*="cookie-consent"], [id*="cookieconsent"], [id*="cookie_consent"],
    [id*="cookie-notice"], [id*="cookienotice"],
    [id*="cookie-popup"], [id*="cookie-modal"],
    [id*="gdpr"], [class*="gdpr"],
    [id*="privacy-popup"], [class*="privacy-popup"],
    /* Overlay backdrops */
    .cookie-overlay, .gdpr-overlay, .consent-overlay,
    /* Didomi */
    #didomi-host, #didomi-popup,
    /* Osano */
    .osano-cm-window, .osano-cm-dialog,
    /* Termly */
    #termly-code-snippet-support,
    /* Iubenda */
    #iubenda-cs-banner,
    /* Usercentrics */
    [data-testid="uc-banner"] {
      display: none !important;
      visibility: hidden !important;
      opacity: 0 !important;
      pointer-events: none !important;
    }
    /* unfreeze scroll that banners lock */
    body.has-overlay, body.modal-open, body.overflow-hidden,
    html.has-overlay, html.overflow-hidden {
      overflow: auto !important;
    }
  `;

  const styleEl = document.createElement('style');
  styleEl.textContent = STYLE;
  document.documentElement.appendChild(styleEl);

  // ── Click "reject" / "close" before user sees it ───────────────────────
  const REJECT_PATTERNS = [
    /^reject\s*(all)?$/i,
    /^decline(\s+all)?$/i,
    /^no[,.]?\s*thank/i,
    /^refuse$/i,
    /^only\s+necessary/i,
    /^necessary\s+only/i,
    /^essential\s+only/i,
    /^save\s+(my\s+)?settings$/i,
    /^continue\s+without/i,
  ];

  const CLOSE_PATTERNS = [
    /^close$/i,
    /^dismiss$/i,
    /^got\s+it$/i,
    /^ok$/i,
    /^×$/,
    /^✕$/,
  ];

  function tryClick(el) {
    if (!el || el.offsetParent === null) return false;
    el.click();
    return true;
  }

  function findAndClick(patterns) {
    const candidates = document.querySelectorAll(
      'button, [role="button"], a.btn, a[class*="button"]'
    );
    for (const el of candidates) {
      const text = (el.innerText || el.textContent || el.value || '').trim();
      for (const pat of patterns) {
        if (pat.test(text)) {
          if (tryClick(el)) return true;
        }
      }
    }
    return false;
  }

  function killBanners() {
    // prefer reject over close — don't silently accept
    if (!findAndClick(REJECT_PATTERNS)) {
      findAndClick(CLOSE_PATTERNS);
    }
  }

  // run immediately, then watch for dynamic banners
  killBanners();

  const observer = new MutationObserver(() => killBanners());
  observer.observe(document.documentElement, { childList: true, subtree: true });

  // stop watching after 10s — banners don't appear that late
  setTimeout(() => observer.disconnect(), 10000);
})();
