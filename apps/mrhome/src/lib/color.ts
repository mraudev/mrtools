/** Colour conversions: Hue uses CIE xy and mirek, Govee RGB and Kelvin, the UI CSS colours. */

export type Rgb = [number, number, number];

const clamp = (v: number, min = 0, max = 1) => Math.min(max, Math.max(min, v));

/** h 0–360, s and v 0–1. */
export function hsvToRgb(h: number, s: number, v: number): Rgb {
  const f = (n: number) => {
    const k = (n + h / 60) % 6;
    return v - v * s * Math.max(0, Math.min(k, 4 - k, 1));
  };
  return [f(5), f(3), f(1)].map((c) => Math.round(c * 255)) as Rgb;
}

const toLinear = (c: number) => (c > 0.04045 ? ((c + 0.055) / 1.055) ** 2.4 : c / 12.92);
const toGamma = (c: number) => (c <= 0.0031308 ? 12.92 * c : 1.055 * c ** (1 / 2.4) - 0.055);

/** sRGB → CIE xy (Philips' wide gamut conversion, as in the Hue developer docs). */
export function rgbToXy([r, g, b]: Rgb): [number, number] {
  const [R, G, B] = [r, g, b].map((c) => toLinear(c / 255));
  const X = R * 0.664511 + G * 0.154324 + B * 0.162028;
  const Y = R * 0.283881 + G * 0.668433 + B * 0.047685;
  const Z = R * 0.000088 + G * 0.07231 + B * 0.986039;
  const sum = X + Y + Z;
  if (sum === 0) return [0.3227, 0.329];
  return [Number((X / sum).toFixed(4)), Number((Y / sum).toFixed(4))];
}

/** CIE xy → sRGB at full brightness, for showing a Hue light's colour. */
export function xyToRgb([x, y]: [number, number]): Rgb {
  if (y <= 0) return [255, 255, 255];
  const z = 1 - x - y;
  const X = x / y;
  const Z = z / y;
  let r = X * 1.656492 - 0.354851 - Z * 0.255038;
  let g = -X * 0.707196 + 1.655397 + Z * 0.036152;
  let b = X * 0.051713 - 0.121364 + Z * 1.01153;
  const max = Math.max(r, g, b, 1e-9);
  [r, g, b] = [r, g, b].map((c) => clamp(toGamma(clamp(c / max))));
  return [r, g, b].map((c) => Math.round(c * 255)) as Rgb;
}

export const mirekToKelvin = (mirek: number) => Math.round(1_000_000 / mirek);
export const kelvinToMirek = (kelvin: number) => Math.round(1_000_000 / kelvin);

/** Approximate colour of white light at `kelvin` (Tanner Helland's fit), for display only. */
export function kelvinToRgb(kelvin: number): Rgb {
  const t = kelvin / 100;
  const r = t <= 66 ? 255 : 329.698727446 * (t - 60) ** -0.1332047592;
  const g = t <= 66 ? 99.4708025861 * Math.log(t) - 161.1195681661 : 288.1221695283 * (t - 60) ** -0.0755148492;
  const b = t >= 66 ? 255 : t <= 19 ? 0 : 138.5177312231 * Math.log(t - 10) - 305.0447927307;
  return [r, g, b].map((c) => Math.round(clamp(c, 0, 255))) as Rgb;
}

export const css = ([r, g, b]: Rgb) => `rgb(${r} ${g} ${b})`;

/** Colours offered as one-click choices. */
export const PALETTE: Rgb[] = [
  [255, 59, 48],
  [255, 149, 0],
  [255, 204, 0],
  [52, 199, 89],
  [0, 199, 190],
  [0, 122, 255],
  [88, 86, 214],
  [175, 82, 222],
  [255, 45, 85],
];
