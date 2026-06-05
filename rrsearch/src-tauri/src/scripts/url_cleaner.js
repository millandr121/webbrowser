(function () {
  'use strict';

  // strip these tracking params from any URL
  const STRIP_PARAMS = new Set([
    // Google
    'utm_source','utm_medium','utm_campaign','utm_term','utm_content',
    'utm_id','utm_reader','utm_name','utm_brand','utm_network','utm_device',
    'gclid','gclsrc','gbraid','wbraid','dclid',
    // Facebook/Meta
    'fbclid','fb_action_ids','fb_action_types','fb_source','fb_ref',
    // Microsoft
    'msclkid',
    // Twitter/X
    'twclid',
    // HubSpot
    'hsa_acc','hsa_cam','hsa_grp','hsa_ad','hsa_src','hsa_tgt',
    'hsa_kw','hsa_mt','hsa_net','hsa_ver',
    '_hsenc','_hsmi',
    // Mailchimp
    'mc_cid','mc_eid',
    // Drip
    '__s',
    // Marketo
    'mkt_tok',
    // Other
    'icid','ncid','cmpid','cid','sc_cid','linkId','ref_',
    'source','medium','campaign',
  ]);

  // redirect unwrappers — returns the real URL or null
  const REDIRECT_RULES = [
    // Google
    { host: 'google.com',   param: 'q' },
    { host: 'google.com',   param: 'url' },
    // t.co (Twitter)
    { host: 't.co',         followHref: true },
    // Facebook
    { host: 'facebook.com', param: 'u', path: '/l.php' },
    { host: 'l.facebook.com', param: 'u' },
    // LinkedIn
    { host: 'linkedin.com', param: 'url', path: '/redir/redirect' },
    // Reddit
    { host: 'out.reddit.com', param: 'url' },
    // YouTube redirect
    { host: 'youtube.com',  param: 'q', path: '/redirect' },
    // Generic /redirect?url=
    { path: '/redirect', param: 'url' },
    { path: '/go',       param: 'url' },
    { path: '/out',      param: 'url' },
    { path: '/click',    param: 'url' },
  ];

  function stripTracking(url) {
    try {
      const u = new URL(url);
      let changed = false;
      for (const key of [...u.searchParams.keys()]) {
        if (STRIP_PARAMS.has(key) || key.startsWith('utm_')) {
          u.searchParams.delete(key);
          changed = true;
        }
      }
      return changed ? u.toString() : url;
    } catch {
      return url;
    }
  }

  function unwrapRedirect(url) {
    try {
      const u = new URL(url);
      for (const rule of REDIRECT_RULES) {
        const hostMatch = !rule.host || u.hostname.endsWith(rule.host);
        const pathMatch = !rule.path  || u.pathname.startsWith(rule.path);
        if (hostMatch && pathMatch && rule.param) {
          const target = u.searchParams.get(rule.param);
          if (target) {
            try { return decodeURIComponent(target); } catch { return target; }
          }
        }
      }
    } catch { /* not a URL */ }
    return null;
  }

  function cleanUrl(url) {
    const unwrapped = unwrapRedirect(url);
    const base = unwrapped || url;
    return stripTracking(base);
  }

  // clean all links on the page
  function cleanLinks() {
    document.querySelectorAll('a[href]').forEach(a => {
      const cleaned = cleanUrl(a.href);
      if (cleaned !== a.href) a.href = cleaned;
    });
  }

  cleanLinks();

  // watch for new links added dynamically
  const obs = new MutationObserver(cleanLinks);
  obs.observe(document.body || document.documentElement, {
    childList: true,
    subtree: true,
    attributeFilter: ['href'],
  });

  // expose for Tauri to call directly on navigation
  window.__rrsearch = window.__rrsearch || {};
  window.__rrsearch.cleanUrl = cleanUrl;
})();
