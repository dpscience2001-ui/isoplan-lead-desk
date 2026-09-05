export function protectCsvCell(value: string): string {
  return /^[\s]*[=+\-@]/.test(value) ? `'${value}` : value;
}
