import { z } from "zod";

const supportedCountries = ["GB", "US", "FR"] as const;

export const leadInputSchema = z.object({
  companyName: z.string().trim().min(2).max(200),
  officialWebsite: z.string().trim().url().max(2_048),
  country: z.enum(supportedCountries),
  language: z.enum(["en", "fr"]),
  cityOrServiceArea: z.string().trim().max(200).optional(),
});

export type LeadInput = z.infer<typeof leadInputSchema>;

export function normalizeOfficialWebsite(value: string): string {
  const url = new URL(value.trim());
  if (url.protocol !== "https:" && url.protocol !== "http:") {
    throw new Error("Only public HTTP or HTTPS websites are supported.");
  }
  url.hash = "";
  url.hostname = url.hostname.toLowerCase();
  if (url.pathname === "/") url.pathname = "";
  return url.toString().replace(/\/$/, "");
}

export function normalizedDomain(value: string): string {
  const hostname = new URL(normalizeOfficialWebsite(value)).hostname;
  return hostname.startsWith("www.") ? hostname.slice(4) : hostname;
}

