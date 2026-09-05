import { describe, expect, it } from "vitest";
import { gmailComposeUrl, isBusinessEmail } from "./gmail-compose";

describe("Gmail compose preparation", () => {
  it("encodes the recipient, subject, and multiline body", () => {
    const url = new URL(gmailComposeUrl("hello@example.com", "Floor plans & layouts", "Hello,\nBonjour."));
    expect(url.origin).toBe("https://mail.google.com");
    expect(url.searchParams.get("to")).toBe("hello@example.com");
    expect(url.searchParams.get("su")).toBe("Floor plans & layouts");
    expect(url.searchParams.get("body")).toBe("Hello,\nBonjour.");
  });
  it.each(["missing", "a@b", "a b@example.com", "@example.com"])("rejects invalid recipient %s", value => {
    expect(isBusinessEmail(value)).toBe(false);
  });
});
