export type LeadStatus =
  | "discovered"
  | "researching"
  | "needs_review"
  | "qualified"
  | "rejected"
  | "draft_ready"
  | "approved"
  | "gmail_opened"
  | "contacted"
  | "follow_up_due"
  | "replied"
  | "interested"
  | "not_interested"
  | "converted"
  | "do_not_contact"
  | "archived";

export interface DashboardCounts {
  total: number;
  needsReview: number;
  qualified: number;
  draftsReady: number;
  contacted: number;
  followUpsDue: number;
  replied: number;
  interested: number;
  converted: number;
}

export interface LeadSummary {
  id: string;
  companyName: string;
  officialWebsite: string;
  country: "GB" | "US" | "FR";
  language: "en" | "fr";
  cityOrServiceArea?: string;
  status: LeadStatus;
  qualificationScore?: number;
  confidenceLevel?: "low" | "medium" | "high";
  createdAt: string;
}

export interface EvidenceClaim {
  id: string;
  classification: "verified_fact" | "extracted_statement" | "ai_inference" | "missing_information" | "user_entered";
  category: string;
  claim: string;
  supportsQualification: -1 | 0 | 1;
  confidence: "low" | "medium" | "high";
  sourceUrl?: string;
  createdAt: string;
}

export interface ReviewData {
  evidence: EvidenceClaim[];
  notes: Array<{ id: string; body: string; createdAt: string }>;
}

export interface ContactRecord {
  id: string;
  contactType: "role_email" | "named_email" | "contact_form" | "phone";
  value: string;
  recipientRole?: string;
  sourceUrl: string;
  cautionReason?: string;
}

export interface OutreachDraft {
  id: string;
  leadId: string;
  contactId?: string;
  recipient?: string;
  language: "en" | "fr";
  subject: string;
  body: string;
  englishTranslation?: string;
  approvedAt?: string;
  gmailOpenedAt?: string;
  sentConfirmedAt?: string;
  updatedAt: string;
}
