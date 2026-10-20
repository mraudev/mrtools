import { describe, expect, it } from "vitest";
import { betrag, datum, htmlText, menge, parseZahl, rechne } from "./format";

describe("format", () => {
  it("formatiert deutsch", () => {
    expect(betrag(1234.5)).toBe("1.234,50");
    expect(menge(12.5)).toBe("12,50");
    expect(menge(1.125)).toBe("1,125");
    expect(datum("2024-06-21")).toBe("21.06.2024");
    expect(datum("")).toBe("");
  });

  it("liest Zahlen und einfache Rechnungen", () => {
    expect(parseZahl("1.234,56")).toBe(1234.56);
    expect(parseZahl("12,5")).toBe(12.5);
    expect(parseZahl("7.5")).toBe(7.5);
    expect(parseZahl("12+12+9")).toBe(33);
    expect(parseZahl("9,85*6,02")).toBe(59.297);
    expect(parseZahl("(2+4)/2")).toBe(3);
    expect(parseZahl("abc")).toBe(0);
    expect(parseZahl("")).toBe(0);
    expect(rechne("1/0")).toBeNull();
  });

  it("macht aus HTML Klartext", () => {
    expect(htmlText("<p>Für <strong>Maler</strong> &amp; mehr</p><p>B</p>")).toBe("Für Maler & mehr\nB");
  });
});
