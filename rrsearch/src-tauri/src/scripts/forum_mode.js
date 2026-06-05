(function () {
  'use strict';

  const host = location.hostname.replace(/^www\./, '');

  // ── Shared: inject clean reading CSS ────────────────────────────────────
  function injectStyle(css) {
    const s = document.createElement('style');
    s.id = 'rrsearch-forum-mode';
    s.textContent = css;
    document.documentElement.appendChild(s);
  }

  // ── Reddit (old + new) ───────────────────────────────────────────────────
  function cleanReddit() {
    injectStyle(`
      /* hide sidebar ads, promoted posts, award noise */
      [data-testid="ad-comments-placement"],
      [data-testid="ad-post-placement"],
      [data-testid="recommended-subreddits"],
      [data-testid="frontpage-sidebar"],
      .promotedlink, .promoted-icon,
      shreddit-ad-post,
      [data-ks-id*="promoted"],
      /* hide "Get Reddit Premium" nags */
      #premium-banner, .premium-banner, [href*="premium"],
      /* hide awards clutter in comments */
      .icon-award, [class*="awardsRow"],
      /* hide infinite scroll "See more" nag */
      .ListingLayout-outerContainer > div:last-child > div[class*="bottom"],
      /* new reddit bloat */
      [data-testid="subreddit-sidebar"],
      #right-sidebar-container,
      .sidebar-grid,
      reddit-recent-pages,
      /* cookie/login nag bar */
      #SHORTCUT_FOCUSABLE_DIV > div[data-testid="bottom-bar"] { display: none !important; }

      /* widen the content column */
      .ListingLayout-backgroundContainer { max-width: 100% !important; }
      .Post { max-width: 740px !important; margin: 0 auto !important; }

      /* expand all collapsed comments */
      .comment.collapsed > .entry { display: block !important; }

      /* cleaner comment thread lines */
      .comment { border-left: 2px solid #222 !important; margin-left: 8px !important; }

      /* remove promoted/ad label noise */
      [class*="sponsored"], [data-click-id="body"][href*="redd.it/events"] { display: none !important; }
    `);

    // auto-expand collapsed comments
    function expandCollapsed() {
      document.querySelectorAll('.comment.collapsed .expand').forEach(btn => btn.click());
    }
    expandCollapsed();
    const obs = new MutationObserver(expandCollapsed);
    obs.observe(document.body, { childList: true, subtree: true });
  }

  // ── Hacker News ─────────────────────────────────────────────────────────
  function cleanHN() {
    injectStyle(`
      body { background: #f6f6ef; font-family: 'Space Mono', monospace; }

      /* widen layout */
      body > center, #hnmain { width: 100% !important; max-width: 860px !important; margin: 0 auto !important; }

      /* bigger, more readable text */
      .comment, .commtext { font-size: 14px !important; line-height: 1.7 !important; }
      .title a { font-size: 15px !important; }

      /* hide vote arrows clutter on hover, clean look */
      .votearrow { opacity: 0.3; }
      .votearrow:hover { opacity: 1; }

      /* indent thread lines */
      .comment td:first-child { border-left: 2px solid #ddd; }
    `);

    // auto-expand [flagged] and [dead] items
    document.querySelectorAll('.coll a[href^="item"]').forEach(a => {
      const toggle = a.closest('tr')?.querySelector('.togg');
      if (toggle) toggle.click();
    });
  }

  // ── Stack Overflow / Stack Exchange ─────────────────────────────────────
  function cleanStackOverflow() {
    injectStyle(`
      /* hide right sidebar */
      #sidebar, .s-sidebar, #feed-link { display: none !important; }
      /* widen content */
      #content, .container { max-width: 100% !important; }
      #question-page .question-page-wrapper { max-width: 860px !important; margin: 0 auto !important; }
      /* hide hot network questions */
      .s-sidebarwidget--hot-network-questions { display: none !important; }
      /* hide "sign up" overlay */
      .js-dismissable-hero, .hero-announcement { display: none !important; }
    `);
  }

  // ── Generic forum cleanup ────────────────────────────────────────────────
  function cleanGenericForum() {
    injectStyle(`
      /* hide common ad placements */
      [id*="advert"], [class*="advert"],
      [id*="banner-ad"], [class*="banner-ad"],
      [id*="sidebar-ad"], [class*="sidebar-ad"],
      [id*="sponsor"], [class*="sponsored"],
      iframe[src*="ads"], iframe[src*="doubleclick"],
      /* improve readability */
      .forum-post, .post-body, .message-body {
        font-size: 14px !important;
        line-height: 1.7 !important;
        max-width: 720px !important;
      }
    `);
  }

  // ── Route by host ────────────────────────────────────────────────────────
  if (host === 'reddit.com' || host.endsWith('.reddit.com')) {
    cleanReddit();
  } else if (host === 'news.ycombinator.com') {
    cleanHN();
  } else if (host === 'stackoverflow.com' || host.endsWith('.stackexchange.com')) {
    cleanStackOverflow();
  } else if (/\b(forum|forums|community|discuss|board|bbs)\b/.test(host)) {
    cleanGenericForum();
  }

  // expose toggle for Tauri command
  window.__rrsearch = window.__rrsearch || {};
  window.__rrsearch.forumModeActive = true;
})();
