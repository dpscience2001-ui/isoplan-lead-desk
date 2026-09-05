import { invoke } from "@tauri-apps/api/core";
import type { ContactRecord, DashboardCounts, LeadSummary, OutreachDraft, ReviewData } from "../types";
import type { LeadInput } from "./lead-validation";

export const emptyDashboardCounts: DashboardCounts = {
  total: 0,
  needsReview: 0,
  qualified: 0,
  draftsReady: 0,
  contacted: 0,
  followUpsDue: 0,
  replied: 0,
  interested: 0,
  converted: 0,
};

export async function loadLeads(): Promise<LeadSummary[]> {
  return invoke<LeadSummary[]>("list_leads");
}

export async function loadDashboardCounts(): Promise<DashboardCounts> {
  return invoke<DashboardCounts>("dashboard_counts");
}

export async function addLead(input: LeadInput): Promise<LeadSummary> {
  return invoke<LeadSummary>("create_lead", { input });
}

export async function setLeadStatus(id: string, status: string, reason?: string): Promise<void> {
  return invoke<void>("set_lead_status", { id, status, reason });
}

export async function loadReviewData(leadId: string): Promise<ReviewData> {
  return invoke<ReviewData>("review_data", { leadId });
}

export async function addNote(leadId: string, body: string): Promise<void> {
  return invoke<void>("add_note", { leadId, body });
}

export async function addEvidence(input: {
  leadId: string; classification: string; category: string; claim: string;
  supportsQualification: -1 | 0 | 1; confidence: string; sourceUrl?: string;
}): Promise<void> {
  return invoke<void>("add_evidence", { input });
}

export async function loadContacts(leadId: string): Promise<ContactRecord[]> {
  return invoke<ContactRecord[]>("list_contacts", { leadId });
}

export async function addContact(input: { leadId: string; contactType: string; value: string; recipientRole?: string; sourceUrl: string }): Promise<void> {
  return invoke<void>("add_contact", { input });
}

export async function getSettings(): Promise<Record<string, string>> {
  return invoke<Record<string, string>>("get_settings");
}

export async function updateSetting(key: string, value: string): Promise<void> {
  return invoke<void>("update_setting", { key, value });
}

export async function createBackup(): Promise<string> {
  return invoke<string>("create_backup");
}

export async function exportLeads(format: "csv" | "json"): Promise<string> {
  return invoke<string>("export_leads", { format });
}

export async function updateLead(input: { id: string; companyName: string; officialWebsite: string; country: string; language: string; cityOrServiceArea?: string }): Promise<LeadSummary> {
  return invoke<LeadSummary>("update_lead", { input });
}
export async function deleteLead(id: string, confirmation: string): Promise<void> { return invoke("delete_lead", { id, confirmation }); }

export async function getDraftSeed(leadId: string): Promise<{ subject: string; body: string }> {
  return invoke("draft_seed", { leadId });
}

export async function getLatestDraft(leadId: string): Promise<OutreachDraft | null> {
  return invoke("latest_draft", { leadId });
}

export async function saveDraft(input: { id?: string; leadId: string; contactId: string; language: string; subject: string; body: string; englishTranslation?: string }): Promise<OutreachDraft> {
  return invoke("save_draft", { input });
}

export async function approveDraft(id: string): Promise<OutreachDraft> {
  return invoke("approve_draft", { id });
}
export async function openGmail(id: string): Promise<void> { return invoke("open_gmail", { id }); }
export async function confirmSent(id: string, sent: boolean): Promise<void> { return invoke("confirm_sent", { id, sent }); }

export interface FollowUpSummary { id:string; leadId:string; companyName:string; dueAt:string; status:string; recipient?:string }
export async function getFollowUps(): Promise<FollowUpSummary[]> { return invoke("due_follow_ups"); }
export async function snoozeFollowUp(id:string, days:number): Promise<void> { return invoke("snooze_follow_up", { id, days }); }
export async function markReplied(leadId:string): Promise<void> { return invoke("mark_replied", { leadId }); }

export interface BackupInfo { fileName:string; sizeBytes:number }
export async function listBackups(): Promise<BackupInfo[]> { return invoke("list_backups"); }
export async function restoreBackup(fileName:string, confirmation:string): Promise<void> { return invoke("restore_backup", { fileName, confirmation }); }
