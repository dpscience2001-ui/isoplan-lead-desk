export const scoringWeights = {
  relevant_business_type: 20,
  portfolio_size: 15,
  public_business_contact: 10,
  no_floor_plan_found: 15,
  weak_2d_floor_plan: 15,
  multi_unit: 15,
  premium_marketing: 5,
  geographic_match: 5,
  repeat_work_likelihood: 10,
  excellent_3d_floor_plans: 25,
  individual_host: 40,
  no_lawful_contact: 25,
} as const;

export type ScoringCategory = keyof typeof scoringWeights;
export type EvidenceDirection = -1 | 0 | 1;

export interface ScoringEvidence {
  category: ScoringCategory;
  supportsQualification: EvidenceDirection;
  confidence: "low" | "medium" | "high";
}

export interface ScoreResult {
  total: number;
  confidence: "low" | "medium" | "high";
  contributions: Array<{ category: ScoringCategory; points: number }>;
}

export function calculateLeadScore(evidence: ScoringEvidence[]): ScoreResult {
  const strongest = new Map<ScoringCategory, EvidenceDirection>();
  for (const item of evidence) {
    const current = strongest.get(item.category) ?? 0;
    if (Math.abs(item.supportsQualification) >= Math.abs(current)) strongest.set(item.category, item.supportsQualification);
  }
  const contributions = [...strongest].map(([category, direction]) => ({ category, points: scoringWeights[category] * direction }));
  const total = Math.max(0, Math.min(100, contributions.reduce((sum, item) => sum + item.points, 0)));
  const highConfidence = evidence.filter(item => item.confidence === "high").length;
  const confidence = evidence.length >= 5 && highConfidence >= 3 ? "high" : evidence.length >= 3 ? "medium" : "low";
  return { total, confidence, contributions };
}

