import { describe, expect, it } from "vitest";
import { mapTrackerRow, parseCsv, rowsToRecords } from "./spreadsheet";

describe("spreadsheet import", () => {
  it("parses quoted CSV fields safely", () => {
    const rows = parseCsv('Companyname,Website,Notes\r\n"Stay, Ltd",stay.test,"Line ""one"""');
    expect(rows[1]).toEqual(["Stay, Ltd", "stay.test", 'Line "one"']);
  });

  it("maps the supplied tracker headings", () => {
    const [row] = rowsToRecords(parseCsv("Lead ID,Companyname,Website,Email\n1,Maison Stays,maison.example,hello@maison.example"));
    expect(mapTrackerRow(row)).toEqual({ companyName: "Maison Stays", officialWebsite: "https://maison.example", country: "GB", language: "en", cityOrServiceArea: undefined });
  });

  it("rejects malformed quoted CSV", () => {
    expect(() => parseCsv('Company,Website\n"Broken,example.com')).toThrow("unclosed quoted field");
  });
});
