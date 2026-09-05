import { describe, expect, it } from "vitest";
import { protectCsvCell } from "./export-safety";

describe("CSV export safety", () => {
  it.each(["=2+2", "+cmd", "-10+20", "@SUM(A1)", "  =IMPORTDATA(x)"])("neutralizes formula-like value %s", value => {
    expect(protectCsvCell(value).startsWith("'")).toBe(true);
  });
  it("does not alter normal business text", () => expect(protectCsvCell("Maison Stays")).toBe("Maison Stays"));
});
