export interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

/** Worst aspect ratio of a row of areas laid along a side of length `side`. */
function worst(row: number[], side: number): number {
  const sum = row.reduce((a, b) => a + b, 0);
  const s2 = side * side;
  return Math.max((s2 * Math.max(...row)) / (sum * sum), (sum * sum) / (s2 * Math.min(...row)));
}

/**
 * Squarified treemap (Bruls, Huizing, van Wijk): splits `rect` into one rectangle per value,
 * proportional to it. `values` must be sorted descending; the result has the same order.
 */
export function squarify(values: number[], rect: Rect): Rect[] {
  const total = values.reduce((a, b) => a + b, 0);
  if (total <= 0 || rect.w <= 0 || rect.h <= 0) return values.map(() => ({ x: rect.x, y: rect.y, w: 0, h: 0 }));

  const scale = (rect.w * rect.h) / total;
  const areas = values.map((v) => v * scale);
  const result: Rect[] = [];
  let { x, y, w, h } = rect;
  let i = 0;
  while (i < areas.length) {
    const side = Math.min(w, h);
    const row = [areas[i]];
    let j = i + 1;
    while (j < areas.length && worst([...row, areas[j]], side) <= worst(row, side)) row.push(areas[j++]);
    const rowSum = row.reduce((a, b) => a + b, 0);

    if (w >= h) {
      // Column on the left.
      const width = h > 0 ? rowSum / h : 0;
      let cy = y;
      for (const area of row) {
        const height = width > 0 ? area / width : 0;
        result.push({ x, y: cy, w: width, h: height });
        cy += height;
      }
      x += width;
      w -= width;
    } else {
      // Row at the top.
      const height = w > 0 ? rowSum / w : 0;
      let cx = x;
      for (const area of row) {
        const width = height > 0 ? area / height : 0;
        result.push({ x: cx, y, w: width, h: height });
        cx += width;
      }
      y += height;
      h -= height;
    }
    i = j;
  }
  return result;
}
