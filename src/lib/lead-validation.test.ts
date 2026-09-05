import { describe, expect, it } from "vitest";
import { leadInputSchema, normalizedDomain, normalizeOfficialWebsite } from "./lead-validation";

describe("lead input validation", () => {
  it("normalizes an official website for duplicate comparison", () => {
    expect(normalizeOfficialWebsite(" HTTPS://WWW.Example.com/#about ")).toBe(
      "https://www.example.com",
    );
    expect(normalizedDomain("https://www.example.com/contact")).toBe("example.com");
  });

  it("accepts only the initial supported markets", () => {
    expect(
      leadInputSchema.safeParse({
        companyName: "Example Stays",
        officialWebsite: "https://example.com",
        country: "GB",
        language: "en",
      }).success,
    ).toBe(true);
    expect(
      leadInputSchema.safeParse({
        companyName: "Example Stays",
        officialWebsite: "https://example.com",
        country: "NL",
        language: "en",
      }).success,
    ).toBe(false);
  });

  it("rejects non-web URL schemes", () => {
    expect(() => normalizeOfficialWebsite("file:///C:/private.txt")).toThrow(
      "Only public HTTP or HTTPS websites are supported.",
    );
  });
});

