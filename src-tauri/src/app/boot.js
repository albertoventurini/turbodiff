// Runs in every window before the page's own scripts (app/boot.rs). Sets the
// theme and mode on <html> before the first paint, so the window never shows
// the wrong one, and leaves the settings for the frontend to start from.
(function () {
  var settings = __SETTINGS__;
  window.__TURBODIFF_SETTINGS__ = settings;
  function apply() {
    var dark =
      settings.appearance === 'dark' ||
      (settings.appearance === 'system' &&
        window.matchMedia('(prefers-color-scheme: dark)').matches);
    var root = document.documentElement;
    root.dataset.theme = settings.theme;
    root.dataset.mode = dark ? 'dark' : 'light';
  }
  if (document.documentElement) {
    apply();
  } else {
    new MutationObserver(function (_, observer) {
      if (document.documentElement) {
        observer.disconnect();
        apply();
      }
    }).observe(document, { childList: true });
  }
})();
