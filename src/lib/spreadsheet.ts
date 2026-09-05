import { strFromU8, unzipSync } from "fflate";

export type SpreadsheetRow = Record<string, string>;

const MAX_FILE_BYTES = 10 * 1024 * 1024;
const MAX_ROWS = 2_000;
const MAX_COLUMNS = 100;

export async function readSpreadsheet(file: File): Promise<SpreadsheetRow[]> {
  if (file.size > MAX_FILE_BYTES) throw new Error("Import files must be 10 MB or smaller.");
  const extension = file.name.split(".").pop()?.toLowerCase();
  if (extension === "csv") return rowsToRecords(parseCsv(await file.text()));
  if (extension === "xlsx") return readXlsx(new Uint8Array(await file.arrayBuffer()));
  throw new Error("Choose a .csv or .xlsx file.");
}

export function parseCsv(text: string): string[][] {
  if (text.includes("\0")) throw new Error("The CSV contains invalid null characters.");
  const rows: string[][] = [];
  let row: string[] = [], field = "", quoted = false;
  for (let index = 0; index < text.length; index += 1) {
    const char = text[index];
    if (quoted) {
      if (char === '"' && text[index + 1] === '"') { field += '"'; index += 1; }
      else if (char === '"') quoted = false;
      else field += char;
    } else if (char === '"' && field === "") quoted = true;
    else if (char === ",") { row.push(field); field = ""; }
    else if (char === "\n") { row.push(field); rows.push(row); row = []; field = ""; }
    else if (char !== "\r") field += char;
  }
  if (quoted) throw new Error("The CSV contains an unclosed quoted field.");
  if (field || row.length) { row.push(field); rows.push(row); }
  return rows.slice(0, MAX_ROWS + 1).map(values => values.slice(0, MAX_COLUMNS));
}

export function rowsToRecords(rows: string[][]): SpreadsheetRow[] {
  if (!rows.length) return [];
  const headers = rows[0].map((value, index) => value.trim() || `Column ${index + 1}`);
  if (new Set(headers.map(normalizeHeader)).size !== headers.length) throw new Error("The spreadsheet contains duplicate column headings.");
  return rows.slice(1).filter(row => row.some(Boolean)).map(row => Object.fromEntries(headers.map((header, index) => [header, row[index]?.trim() ?? ""])));
}

export function mapTrackerRow(row: SpreadsheetRow, fallbackCountry: "GB" | "US" | "FR" = "GB") {
  const normalized = Object.fromEntries(Object.entries(row).map(([key, value]) => [normalizeHeader(key), value.trim()]));
  const companyName = pick(normalized, "companyname", "company", "businessname", "name");
  let officialWebsite = pick(normalized, "website", "companywebsite", "officialwebsite", "url");
  if (officialWebsite && !officialWebsite.includes("://")) officialWebsite = `https://${officialWebsite}`;
  const countryText = pick(normalized, "country", "market").toLowerCase();
  const country = countryText.includes("france") || countryText === "fr" ? "FR" : countryText.includes("united states") || countryText === "us" || countryText === "usa" ? "US" : countryText.includes("united kingdom") || countryText === "uk" || countryText === "gb" ? "GB" : fallbackCountry;
  return { companyName, officialWebsite, country: country as "GB" | "US" | "FR", language: country === "FR" ? "fr" as const : "en" as const, cityOrServiceArea: pick(normalized, "city", "region", "servicearea") || undefined };
}

function readXlsx(bytes: Uint8Array): SpreadsheetRow[] {
  let files: Record<string, Uint8Array>;
  try { files = unzipSync(bytes); } catch { throw new Error("The XLSX file is damaged or not a valid Excel workbook."); }
  const sheet = files["xl/worksheets/sheet1.xml"];
  if (!sheet) throw new Error("The first Excel worksheet could not be read.");
  const shared = files["xl/sharedStrings.xml"] ? parseSharedStrings(strFromU8(files["xl/sharedStrings.xml"])) : [];
  return rowsToRecords(parseSheet(strFromU8(sheet), shared));
}

function parseSharedStrings(xml: string): string[] {
  return [...xml.matchAll(/<si(?:\s[^>]*)?>([\s\S]*?)<\/si>/g)].map(match =>
    [...match[1].matchAll(/<t(?:\s[^>]*)?>([\s\S]*?)<\/t>/g)].map(part => decodeXml(part[1])).join(""),
  );
}

function parseSheet(xml: string, shared: string[]): string[][] {
  const rows: string[][] = [];
  for (const rowMatch of xml.matchAll(/<row(?:\s[^>]*)?>([\s\S]*?)<\/row>/g)) {
    const row: string[] = [];
    for (const cellMatch of rowMatch[1].matchAll(/<c\s([^>]*)>([\s\S]*?)<\/c>/g)) {
      const attributes = cellMatch[1], content = cellMatch[2];
      const reference = /\br="([A-Z]+)\d+"/.exec(attributes)?.[1];
      if (!reference) continue;
      const column = columnIndex(reference);
      if (column >= MAX_COLUMNS) continue;
      const type = /\bt="([^"]+)"/.exec(attributes)?.[1];
      const raw = /<v(?:\s[^>]*)?>([\s\S]*?)<\/v>/.exec(content)?.[1] ?? /<t(?:\s[^>]*)?>([\s\S]*?)<\/t>/.exec(content)?.[1] ?? "";
      row[column] = type === "s" ? shared[Number(raw)] ?? "" : decodeXml(raw);
    }
    rows.push(row);
    if (rows.length > MAX_ROWS) break;
  }
  return rows;
}

function columnIndex(letters: string): number {
  return [...letters].reduce((value, letter) => value * 26 + letter.charCodeAt(0) - 64, 0) - 1;
}

function decodeXml(value: string): string {
  return value.replace(/&lt;/g, "<").replace(/&gt;/g, ">").replace(/&quot;/g, '"').replace(/&apos;/g, "'").replace(/&amp;/g, "&");
}

function normalizeHeader(value: string): string { return value.toLowerCase().replace(/[^a-z0-9]/g, ""); }
function pick(row: Record<string, string>, ...keys: string[]): string { return keys.map(key => row[key]).find(Boolean) ?? ""; }
