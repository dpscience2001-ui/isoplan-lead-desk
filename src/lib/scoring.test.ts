import { describe, expect, it } from "vitest";
import { calculateLeadScore } from "./scoring";

describe("explainable lead scoring", () => {
  it("returns visible contributions and caps the result at 100", () => {
    const result = calculateLeadScore([
      { category: "relevant_business_type", supportsQualification: 1, confidence: "high" },
      { category: "portfolio_size", supportsQualification: 1, confidence: "high" },
      { category: "public_business_contact", supportsQualification: 1, confidence: "high" },
      { category: "no_floor_plan_found", supportsQualification: 1, confidence: "medium" },
      { category: "multi_unit", supportsQualification: 1, confidence: "high" },
      { category: "repeat_work_likelihood", supportsQualification: 1, confidence: "medium" },
      { category: "premium_marketing", supportsQualification: 1, confidence: "high" },
      { category: "geographic_match", supportsQualification: 1, confidence: "high" },
    ]);
    expect(result.total).toBe(95);
    expect(result.confidence).toBe("high");
    expect(result.contributions).toContainEqual({ category: "multi_unit", points: 15 });
  });

  it("applies negative evidence and never returns below zero", () => {
    expect(calculateLeadScore([{ category: "individual_host", supportsQualification: -1, confidence: "high" }]).total).toBe(0);
  });

  it("counts a category once rather than inflating repeated claims", () => {
    const result = calculateLeadScore([
      { category: "premium_marketing", supportsQualification: 1, confidence: "low" },
      { category: "premium_marketing", supportsQualification: 1, confidence: "high" },
    ]);
    expect(result.total).toBe(5);
  });
});
