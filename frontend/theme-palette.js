/**
 * Theme-aware color palette for dark mode.
 *
 * Most components use Tailwind's default colors directly (bg-white, text-gray-900,
 * bg-blue-50, ...). Instead of adding `dark:` variants to every component, the gray
 * scale and the light/deep accent shades are backed by CSS variables that switch
 * when the `.dark` class is on <html>. Light mode values are Tailwind's defaults,
 * so light mode renders exactly as before.
 */
const plugin = require('tailwindcss/plugin');
const twColors = require('tailwindcss/colors');

const GRAY_SHADES = ['50', '100', '200', '300', '400', '500', '600', '700', '800', '900', '950'];
const ACCENT_FAMILIES = ['blue', 'red', 'green', 'yellow', 'amber', 'orange', 'purple'];

// Light tints (backgrounds, borders) become dark tints of the family's 500 shade.
const TINT_MIX_RATIOS = { '50': 0.12, '100': 0.2, '200': 0.32 };
// Deep shades (text on tints) become their light counterparts.
const DEEP_SHADE_SWAPS = { '800': '200', '900': '100', '950': '50' };
// Mid shades are kept for solid buttons; only their text usage is lightened.
const TEXT_SHADE_SWAPS = { '600': '400', '700': '300' };

const DARK_SURFACE_HEX = '#0b1120';

function hexToRgb(hex) {
  const value = hex.replace('#', '');
  return [0, 2, 4].map((offset) => parseInt(value.slice(offset, offset + 2), 16));
}

function rgbTriplet(rgb) {
  return rgb.join(' ');
}

function mixHex(foreground, background, ratio) {
  const fg = hexToRgb(foreground);
  const bg = hexToRgb(background);
  return fg.map((channel, i) => Math.round(channel * ratio + bg[i] * (1 - ratio)));
}

function varName(family, shade) {
  return `--theme-${family}-${shade}`;
}

function varColor(name) {
  return `rgb(var(${name}) / <alpha-value>)`;
}

function mirrorGrayShade(shade) {
  const index = GRAY_SHADES.indexOf(shade);
  return GRAY_SHADES[GRAY_SHADES.length - 1 - index];
}

const themedAccentShades = [...Object.keys(TINT_MIX_RATIOS), ...Object.keys(DEEP_SHADE_SWAPS)];

const themeColors = {
  gray: Object.fromEntries(GRAY_SHADES.map((shade) => [shade, varColor(varName('gray', shade))])),
  ...Object.fromEntries(
    ACCENT_FAMILIES.map((family) => [
      family,
      Object.fromEntries(themedAccentShades.map((shade) => [shade, varColor(varName(family, shade))])),
    ])
  ),
};

function buildVariables() {
  const light = {};
  const dark = { '--theme-surface': rgbTriplet(hexToRgb(DARK_SURFACE_HEX)) };

  for (const shade of GRAY_SHADES) {
    light[varName('gray', shade)] = rgbTriplet(hexToRgb(twColors.gray[shade]));
    dark[varName('gray', shade)] = rgbTriplet(hexToRgb(twColors.gray[mirrorGrayShade(shade)]));
  }

  for (const family of ACCENT_FAMILIES) {
    const palette = twColors[family];
    for (const [shade, ratio] of Object.entries(TINT_MIX_RATIOS)) {
      light[varName(family, shade)] = rgbTriplet(hexToRgb(palette[shade]));
      dark[varName(family, shade)] = rgbTriplet(mixHex(palette['500'], DARK_SURFACE_HEX, ratio));
    }
    for (const [shade, swap] of Object.entries(DEEP_SHADE_SWAPS)) {
      light[varName(family, shade)] = rgbTriplet(hexToRgb(palette[shade]));
      dark[varName(family, shade)] = rgbTriplet(hexToRgb(palette[swap]));
    }
  }

  return { light, dark };
}

function buildDarkOverrides() {
  const surface = 'rgb(var(--theme-surface) / var(--tw-bg-opacity, 1))';
  const overrides = {
    '.dark .bg-white': { backgroundColor: surface },
    '.dark .bg-white\\/50': { backgroundColor: 'rgb(var(--theme-surface) / 0.5)' },
  };

  for (const family of ACCENT_FAMILIES) {
    for (const [shade, swap] of Object.entries(TEXT_SHADE_SWAPS)) {
      const color = { color: twColors[family][swap] };
      overrides[`.dark .text-${family}-${shade}`] = color;
      overrides[`.dark .hover\\:text-${family}-${shade}:hover`] = color;
    }
  }

  return overrides;
}

const themePalettePlugin = plugin(({ addBase }) => {
  const { light, dark } = buildVariables();
  addBase({ ':root': light, '.dark': dark, ...buildDarkOverrides() });
});

module.exports = { themeColors, themePalettePlugin };
