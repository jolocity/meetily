// Applies the saved theme before first paint so dark mode does not flash light on startup.
// Keep the storage key and values in sync with src/lib/theme.ts.
(function () {
  try {
    var preference = localStorage.getItem('themePreference');
    var prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
    var isDark = preference === 'dark' || (preference === 'system' && prefersDark);
    if (isDark) {
      document.documentElement.classList.add('dark');
      document.documentElement.style.colorScheme = 'dark';
    }
  } catch (e) {
    // Storage unavailable: keep the default light theme.
  }
})();
