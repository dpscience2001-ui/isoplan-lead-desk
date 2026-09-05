import { FormEvent, useCallback, useEffect, useMemo, useState } from "react";
import {
  Ban, BellRing, BriefcaseBusiness, ExternalLink, LayoutDashboard,
  Plus, Search, Settings, SquareKanban, X,
} from "lucide-react";
import { addContact, addEvidence, addLead, addNote, approveDraft, createBackup, emptyDashboardCounts, exportLeads, getDraftSeed, getLatestDraft, getSettings, loadContacts, loadDashboardCounts, loadLeads, loadReviewData, saveDraft, setLeadStatus, updateSetting } from "./lib/desktop-api";
import { leadInputSchema, normalizeOfficialWebsite } from "./lib/lead-validation";
import { mapTrackerRow, readSpreadsheet, type SpreadsheetRow } from "./lib/spreadsheet";
import { calculateLeadScore, type ScoringCategory } from "./lib/scoring";
import type { ContactRecord, DashboardCounts, LeadSummary, OutreachDraft, ReviewData } from "./types";

const navigation = [
  ["Dashboard", LayoutDashboard], ["Find leads", Search],
  ["Review queue", BriefcaseBusiness], ["Pipeline", SquareKanban],
  ["Follow-ups", BellRing], ["Do not contact", Ban], ["Settings", Settings],
] as const;

const countryNames = { GB: "United Kingdom", US: "United States", FR: "France" } as const;

export function App() {
  const [activeView, setActiveView] = useState("Dashboard");
  const [leads, setLeads] = useState<LeadSummary[]>([]);
  const [counts, setCounts] = useState<DashboardCounts>(emptyDashboardCounts);
  const [showAddLead, setShowAddLead] = useState(false);
  const [showImport, setShowImport] = useState(false);
  const [selectedLead, setSelectedLead] = useState<LeadSummary>();
  const [notice, setNotice] = useState<string>();
  const [leadQuery, setLeadQuery] = useState("");
  const [countryFilter, setCountryFilter] = useState<"all" | "GB" | "US" | "FR">("all");

  const refresh = useCallback(async () => {
    try {
      const [nextLeads, nextCounts] = await Promise.all([loadLeads(), loadDashboardCounts()]);
      setLeads(nextLeads); setCounts(nextCounts); setNotice(undefined);
    } catch {
      setNotice("Desktop database is available when the native IsoPlan application is running.");
    }
  }, []);
  useEffect(() => { void refresh(); }, [refresh]);

  const metrics = useMemo(() => [
    ["Awaiting review", counts.needsReview, "Research decisions"],
    ["Qualified", counts.qualified, "Ready for outreach"],
    ["Drafts ready", counts.draftsReady, "Require your approval"],
    ["Follow-ups due", counts.followUpsDue, "Messages needing attention"],
  ] as const, [counts]);
  const visibleLeads = useMemo(() => leads.filter(lead => {
    const query = leadQuery.trim().toLowerCase();
    const matchesText = !query || `${lead.companyName} ${lead.officialWebsite} ${lead.cityOrServiceArea || ""}`.toLowerCase().includes(query);
    return matchesText && (countryFilter === "all" || lead.country === countryFilter);
  }), [leads, leadQuery, countryFilter]);

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand"><div className="brand-mark">I</div><div><strong>IsoPlan</strong><span>Lead Desk</span></div></div>
        <nav aria-label="Primary navigation">
          {navigation.map(([label, Icon]) => (
            <button className={activeView === label ? "nav-item active" : "nav-item"} key={label} onClick={() => setActiveView(label)}>
              <Icon size={17} strokeWidth={1.8} /><span>{label}</span>
              {label === "Review queue" && <small>{counts.needsReview}</small>}
            </button>
          ))}
        </nav>
        <div className="local-status"><span className="status-dot"/><div><strong>Portable workspace</strong><span>Database and cache stay on D:</span></div></div>
      </aside>

      <main>
        <header className="topbar">
          <div><p className="eyebrow">{new Intl.DateTimeFormat("en-GB", { weekday: "long", day: "numeric", month: "long" }).format(new Date())}</p><h1>{activeView}</h1></div>
          <button className="primary-action" onClick={() => setShowAddLead(true)}><Plus size={16}/> Add official website</button>
        </header>

        <section className="workspace">
          {notice && <div className="notice">{notice}</div>}
          {activeView === "Dashboard" ? (
            <>
              <div className="section-heading"><div><h2>Today’s workspace</h2><p>A focused view of leads that need your attention.</p></div><span className="privacy-note">Private · portable database</span></div>
              <div className="metric-grid">{metrics.map(([label, value, note]) => <article className="metric" key={label}><span>{label}</span><strong>{value}</strong><p>{note}</p></article>)}</div>
              <div className="content-grid">
                <section className="panel action-panel"><div className="panel-heading"><h3>{leads.length ? "Recently added" : "Next best action"}</h3><span>{counts.total} total leads</span></div>
                  {leads.length ? <LeadList leads={leads.slice(0, 6)} onSelect={setSelectedLead} /> : <EmptyLeads onAdd={() => setShowAddLead(true)} onImport={() => setShowImport(true)} />}
                </section>
                <aside className="panel principles"><div className="panel-heading"><h3>Research standard</h3></div><ul>
                  <li><span>01</span><div><strong>Official sources</strong><p>Claims retain their exact source and retrieval date.</p></div></li>
                  <li><span>02</span><div><strong>Explainable qualification</strong><p>Scores show evidence, uncertainty, and contrary signals.</p></div></li>
                  <li><span>03</span><div><strong>You remain in control</strong><p>Every message requires review and a deliberate send.</p></div></li>
                </ul></aside>
              </div>
            </>
          ) : activeView === "Settings" ? <SettingsView onNotice={setNotice} /> : (
            <section className="panel"><div className="panel-heading"><h3>{activeView}</h3><span>{visibleLeads.length} records</span></div>
              {activeView !== "Find leads" && <div className="filter-bar"><label><Search size={14}/><input value={leadQuery} onChange={event => setLeadQuery(event.target.value)} placeholder="Search company, website, or area"/></label><select value={countryFilter} onChange={event => setCountryFilter(event.target.value as typeof countryFilter)}><option value="all">All markets</option><option value="GB">United Kingdom</option><option value="US">United States</option><option value="FR">France</option></select></div>}
              {activeView === "Find leads" ? <EmptyLeads onAdd={() => setShowAddLead(true)} onImport={() => setShowImport(true)} /> : <LeadList leads={activeView === "Do not contact" ? visibleLeads.filter(lead => lead.status === "do_not_contact") : visibleLeads} editable={activeView === "Pipeline"} onChanged={refresh} onSelect={setSelectedLead} />}
            </section>
          )}
        </section>
      </main>
      {showAddLead && <AddLeadDialog onClose={() => setShowAddLead(false)} onCreated={async () => { setShowAddLead(false); await refresh(); }} />}
      {showImport && <ImportDialog onClose={() => setShowImport(false)} onImported={async message => { setShowImport(false); setNotice(message); await refresh(); }} />}
      {selectedLead && <ReviewDialog lead={selectedLead} onClose={() => setSelectedLead(undefined)} onChanged={async () => { await refresh(); }} />}
    </div>
  );
}

function EmptyLeads({ onAdd, onImport }: { onAdd: () => void; onImport: () => void }) {
  return <div className="empty-state"><div className="empty-icon"><BriefcaseBusiness size={24}/></div><h4>Build your first review queue</h4><p>Add an official vacation-rental company website. No search, research, or contact happens automatically.</p><div className="button-row"><button className="primary-action" onClick={onAdd}>Add website</button><button className="secondary-action" onClick={onImport}>Import CSV or Excel</button></div></div>;
}

function ImportDialog({ onClose, onImported }: { onClose: () => void; onImported: (message: string) => Promise<void> }) {
  const [rows, setRows] = useState<SpreadsheetRow[]>([]); const [fileName, setFileName] = useState("");
  const [fallbackCountry, setFallbackCountry] = useState<"GB" | "US" | "FR">("GB");
  const [error, setError] = useState<string>(); const [importing, setImporting] = useState(false);
  const mapped = useMemo(() => rows.map(row => mapTrackerRow(row, fallbackCountry)), [rows, fallbackCountry]);
  const valid = mapped.filter(row => leadInputSchema.safeParse(row).success);

  async function choose(file?: File) {
    if (!file) return; setError(undefined); setRows([]); setFileName(file.name);
    try { setRows(await readSpreadsheet(file)); } catch (reason) { setError(reason instanceof Error ? reason.message : "The spreadsheet could not be read."); }
  }
  async function performImport() {
    if (!valid.length) return; setImporting(true); setError(undefined);
    let added = 0, duplicates = 0, failed = mapped.length - valid.length;
    for (const candidate of valid.slice(0, 500)) {
      try { await addLead(candidate); added += 1; }
      catch (reason) { if (String(reason).toLowerCase().includes("already exists")) duplicates += 1; else failed += 1; }
    }
    await onImported(`Import complete: ${added} added, ${duplicates} duplicates skipped, ${failed} invalid or failed.`);
  }
  return <div className="dialog-backdrop" role="presentation"><section className="dialog import-dialog" role="dialog" aria-modal="true" aria-labelledby="import-title"><div className="dialog-heading"><div><p className="eyebrow">Local import</p><h2 id="import-title">Import leads</h2></div><button className="icon-button" onClick={onClose} aria-label="Close"><X size={18}/></button></div><div className="import-body">
    <label className="file-drop"><input type="file" accept=".csv,.xlsx" onChange={event => void choose(event.target.files?.[0])}/><BriefcaseBusiness size={22}/><strong>{fileName || "Choose a CSV or Excel workbook"}</strong><span>Maximum 10 MB · first worksheet only</span></label>
    <label>Fallback market for rows without a country<select value={fallbackCountry} onChange={event => setFallbackCountry(event.target.value as "GB" | "US" | "FR")}><option value="GB">United Kingdom</option><option value="US">United States</option><option value="FR">France</option></select></label>
    {rows.length > 0 && <div className="import-summary"><strong>{rows.length} rows found</strong><span>{valid.length} ready · {rows.length - valid.length} missing or invalid company/website</span><p>Recognized headings include Companyname, Company Website, Website, Country, Market, City, and Region.</p></div>}
    {error && <p className="form-error" role="alert">{error}</p>}
    <div className="dialog-actions"><button className="secondary-action" onClick={onClose}>Cancel</button><button className="primary-action" disabled={!valid.length || importing} onClick={() => void performImport()}>{importing ? "Importing…" : `Import ${valid.length || ""} leads`}</button></div>
  </div></section></div>;
}

function LeadList({ leads, editable = false, onChanged, onSelect }: { leads: LeadSummary[]; editable?: boolean; onChanged?: () => Promise<void>; onSelect?: (lead: LeadSummary) => void }) {
  if (!leads.length) return <div className="compact-empty">No leads in this view.</div>;
  async function changeStatus(lead: LeadSummary, status: string) {
    let reason: string | undefined;
    if (status === "do_not_contact") { reason = window.prompt(`Why should ${lead.companyName} never be contacted?`)?.trim(); if (!reason) return; }
    try { await setLeadStatus(lead.id, status, reason); await onChanged?.(); } catch (error) { window.alert(String(error)); }
  }
  return <div className="lead-list">{leads.map(lead => <article className="lead-row" key={lead.id}><button className="lead-identity lead-button" onClick={() => onSelect?.(lead)}><span className="country-code">{lead.country}</span><div><strong>{lead.companyName}</strong><p>{lead.cityOrServiceArea || countryNames[lead.country]} · {lead.qualificationScore == null ? "Not scored" : `${lead.qualificationScore}/100 · ${lead.confidenceLevel} confidence`}</p></div></button>{editable ? <select className="status-select" value={lead.status} onChange={event => void changeStatus(lead, event.target.value)}><option value="discovered">Discovered</option><option value="needs_review">Needs review</option><option value="qualified">Qualified</option><option value="rejected">Rejected</option><option value="draft_ready">Draft ready</option><option value="contacted">Contacted</option><option value="follow_up_due">Follow-up due</option><option value="replied">Replied</option><option value="interested">Interested</option><option value="not_interested">Not interested</option><option value="converted">Converted</option><option value="do_not_contact">Do not contact</option><option value="archived">Archived</option></select> : <span className={`status status-${lead.status}`}>{lead.status.replaceAll("_", " ")}</span>}<a href={lead.officialWebsite} target="_blank" rel="noreferrer" aria-label={`Open ${lead.companyName}`}><ExternalLink size={15}/></a></article>)}</div>;
}

const evidenceCategories = [
  ["relevant_business_type", "Relevant management business"], ["portfolio_size", "Portfolio approximately 5–50"],
  ["public_business_contact", "Public business contact"], ["no_floor_plan_found", "No floor plan found on analyzed pages"],
  ["weak_2d_floor_plan", "Limited 2D floor-plan presentation"], ["multi_unit", "Multi-unit or whole-property layout"],
  ["premium_marketing", "Premium visual marketing"], ["repeat_work_likelihood", "Repeat-work potential"],
  ["geographic_match", "Supported market"], ["excellent_3d_floor_plans", "Already uses strong 3D plans"],
  ["individual_host", "Individual host rather than business"], ["no_lawful_contact", "No suitable public contact"],
] as const;

function ReviewDialog({ lead, onClose, onChanged }: { lead: LeadSummary; onClose: () => void; onChanged: () => Promise<void> }) {
  const [data, setData] = useState<ReviewData>({ evidence: [], notes: [] }); const [error, setError] = useState<string>();
  const [contacts, setContacts] = useState<ContactRecord[]>([]);
  const [showDraft, setShowDraft] = useState(false);
  const reload = useCallback(async () => { try { const [review, nextContacts] = await Promise.all([loadReviewData(lead.id), loadContacts(lead.id)]); setData(review); setContacts(nextContacts); setError(undefined); } catch (reason) { setError(String(reason)); } }, [lead.id]);
  useEffect(() => { void reload(); }, [reload]);
  const calculated = useMemo(() => calculateLeadScore(data.evidence.map(item => ({ category: item.category as ScoringCategory, supportsQualification: item.supportsQualification, confidence: item.confidence }))), [data.evidence]);
  async function submitEvidence(event: FormEvent<HTMLFormElement>) {
    event.preventDefault(); const form = event.currentTarget; const values = new FormData(form);
    try { await addEvidence({ leadId: lead.id, classification: String(values.get("classification")), category: String(values.get("category")), claim: String(values.get("claim")), supportsQualification: Number(values.get("direction")) as -1|0|1, confidence: String(values.get("confidence")), sourceUrl: String(values.get("sourceUrl") || "") || undefined }); form.reset(); await reload(); await onChanged(); }
    catch (reason) { setError(String(reason)); }
  }
  async function submitNote(event: FormEvent<HTMLFormElement>) { event.preventDefault(); const form = event.currentTarget; const body = String(new FormData(form).get("body") || ""); try { await addNote(lead.id, body); form.reset(); await reload(); } catch (reason) { setError(String(reason)); } }
  async function submitContact(event: FormEvent<HTMLFormElement>) { event.preventDefault(); const form = event.currentTarget; const values = new FormData(form); try { await addContact({ leadId: lead.id, contactType: String(values.get("contactType")), value: String(values.get("value")), recipientRole: String(values.get("recipientRole") || "") || undefined, sourceUrl: String(values.get("sourceUrl")) }); form.reset(); await reload(); } catch (reason) { setError(String(reason)); } }
  return <div className="dialog-backdrop"><section className="dialog review-dialog" role="dialog" aria-modal="true"><div className="dialog-heading"><div><p className="eyebrow">Evidence review</p><h2>{lead.companyName}</h2><a className="source-link" href={lead.officialWebsite} target="_blank" rel="noreferrer">{lead.officialWebsite} <ExternalLink size={12}/></a></div><button className="icon-button" onClick={onClose}><X size={18}/></button></div><div className="review-grid"><div className="review-column"><h3>Add structured evidence</h3><form onSubmit={submitEvidence}>
    <label>Category<select name="category">{evidenceCategories.map(([value,label]) => <option value={value} key={value}>{label}</option>)}</select></label>
    <div className="form-grid"><label>Evidence type<select name="classification"><option value="extracted_statement">Website statement</option><option value="verified_fact">Verified fact</option><option value="ai_inference">AI inference</option><option value="missing_information">Missing information</option><option value="user_entered">User observation</option></select></label><label>Effect<select name="direction"><option value="1">Supports qualification</option><option value="0">Neutral</option><option value="-1">Against qualification</option></select></label></div>
    <label>Observation<textarea name="claim" required minLength={5} maxLength={2000} placeholder="Describe only what is visible or stated."/></label><div className="form-grid"><label>Confidence<select name="confidence"><option value="high">High</option><option value="medium">Medium</option><option value="low">Low</option></select></label><label>Exact source URL<input name="sourceUrl" type="url" required placeholder="https://…"/></label></div><button className="primary-action">Add evidence</button>
  </form><h3 className="subsection-title">Public contact</h3><form onSubmit={submitContact}><div className="form-grid"><label>Type<select name="contactType"><option value="role_email">Role-based email</option><option value="contact_form">Contact form</option><option value="named_email">Named email — caution</option><option value="phone">Phone</option></select></label><label>Email, URL, or phone<input name="value" required/></label></div><div className="form-grid"><label>Recipient role<input name="recipientRole" placeholder="Reservations team"/></label><label>Exact source URL<input name="sourceUrl" type="url" required placeholder="https://…"/></label></div><button className="secondary-action">Add contact</button></form>{contacts.map(contact => <div className="contact-card" key={contact.id}><strong>{contact.value}</strong><span>{contact.contactType.replaceAll("_", " ")}{contact.recipientRole ? ` · ${contact.recipientRole}` : ""}</span>{contact.cautionReason && <p>{contact.cautionReason}</p>}</div>)}<button className="primary-action draft-action" disabled={!contacts.some(contact => contact.contactType.endsWith("email")) || !data.evidence.length} onClick={() => setShowDraft(true)}>Prepare outreach draft</button></div><div className="review-column"><div className="score-block"><span>Current score</span><strong>{data.evidence.length ? calculated.total : "—"}<small>/100</small></strong><p>{data.evidence.length ? `${calculated.confidence} confidence` : "Add evidence to calculate"}</p></div><h3>Evidence</h3><div className="evidence-list">{data.evidence.length ? data.evidence.map(item => <article key={item.id}><div><span className={item.supportsQualification > 0 ? "positive" : item.supportsQualification < 0 ? "negative" : "neutral"}>{item.supportsQualification > 0 ? "+" : item.supportsQualification < 0 ? "−" : "•"}</span><strong>{item.category.replaceAll("_", " ")}</strong></div><p>{item.claim}</p>{item.sourceUrl && <a href={item.sourceUrl} target="_blank" rel="noreferrer">View source</a>}<small>{item.classification.replaceAll("_", " ")} · {item.confidence}</small></article>) : <p className="muted-copy">No evidence recorded yet.</p>}</div><h3>Notes</h3><form className="note-form" onSubmit={submitNote}><textarea name="body" required maxLength={4000} placeholder="Private working note…"/><button className="secondary-action">Add note</button></form>{data.notes.map(note => <p className="saved-note" key={note.id}>{note.body}</p>)}</div></div>{error && <p className="form-error review-error">{error}</p>}{showDraft && <DraftDialog lead={lead} contacts={contacts.filter(contact => contact.contactType.endsWith("email"))} onClose={() => setShowDraft(false)} onChanged={onChanged}/>}</section></div>;
}

function DraftDialog({ lead, contacts, onClose, onChanged }: { lead: LeadSummary; contacts: ContactRecord[]; onClose: () => void; onChanged: () => Promise<void> }) {
  const [draft, setDraft] = useState<OutreachDraft>(); const [seed, setSeed] = useState<{subject:string;body:string}>(); const [error, setError] = useState<string>();
  useEffect(() => { void (async () => { try { const existing = await getLatestDraft(lead.id); if (existing) setDraft(existing); else setSeed(await getDraftSeed(lead.id)); } catch (reason) { setError(String(reason)); } })(); }, [lead.id]);
  async function submit(event: FormEvent<HTMLFormElement>) { event.preventDefault(); const values = new FormData(event.currentTarget); try { const saved = await saveDraft({ id: draft?.id, leadId: lead.id, contactId: String(values.get("contactId")), language: lead.language, subject: String(values.get("subject")), body: String(values.get("body")), englishTranslation: String(values.get("englishTranslation") || "") || undefined }); setDraft(saved); setSeed({ subject: saved.subject, body: saved.body }); await onChanged(); setError(undefined); } catch (reason) { setError(String(reason)); } }
  async function approve() { if (!draft || !window.confirm("Approve this exact recipient, subject, and message? This still will not send anything.")) return; try { setDraft(await approveDraft(draft.id)); await onChanged(); } catch (reason) { setError(String(reason)); } }
  const subject = draft?.subject ?? seed?.subject ?? ""; const body = draft?.body ?? seed?.body ?? "";
  return <div className="dialog-backdrop nested-dialog"><section className="dialog draft-dialog"><div className="dialog-heading"><div><p className="eyebrow">Human review required</p><h2>Outreach draft</h2></div><button className="icon-button" onClick={onClose}><X size={18}/></button></div><form key={draft?.updatedAt || seed?.subject || "loading"} onSubmit={submit}><label>Recipient<select name="contactId" defaultValue={draft?.contactId || contacts[0]?.id}>{contacts.map(contact => <option value={contact.id} key={contact.id}>{contact.value}{contact.cautionReason ? " — caution" : ""}</option>)}</select></label><label>Subject<input name="subject" required maxLength={150} defaultValue={subject}/></label><label>Message<textarea name="body" required minLength={20} maxLength={10000} defaultValue={body}/></label>{lead.language === "fr" && <label>Optional English translation for review<textarea name="englishTranslation" defaultValue={draft?.englishTranslation}/></label>}<div className="form-assurance">Saving an edit removes prior approval. Opening Gmail and confirming a send are separate later actions.</div>{error && <p className="form-error">{error}</p>}<div className="dialog-actions"><button type="button" className="secondary-action" onClick={onClose}>Close</button><button className="secondary-action">Save draft</button><button type="button" className="primary-action" disabled={!draft || Boolean(draft.approvedAt)} onClick={() => void approve()}>{draft?.approvedAt ? "Approved" : "Approve exact draft"}</button></div></form></section></div>;
}

function SettingsView({ onNotice }: { onNotice: (message: string) => void }) {
  const [settings, setSettings] = useState<Record<string, string>>({}); const [error, setError] = useState<string>();
  useEffect(() => { void getSettings().then(setSettings).catch(reason => setError(String(reason))); }, []);
  async function save(event: FormEvent<HTMLFormElement>) { event.preventDefault(); const values = new FormData(event.currentTarget); try { for (const key of ["fiverr_gig_url", "sender_name", "sender_business_name", "sender_postal_address", "follow_up_delay_days", "backup_retention_count"]) await updateSetting(key, String(values.get(key) || "")); setError(undefined); onNotice("Settings saved locally."); } catch (reason) { setError(String(reason)); } }
  async function backup() { try { onNotice(`Backup created on D: at ${await createBackup()}`); } catch (reason) { setError(String(reason)); } }
  async function exportData(format: "csv" | "json") { try { onNotice(`${format.toUpperCase()} export created on D: at ${await exportLeads(format)}`); } catch (reason) { setError(String(reason)); } }
  return <section className="settings-layout"><div className="panel"><div className="panel-heading"><h3>Identity and outreach</h3><span>Stored locally</span></div><form key={settings.fiverr_gig_url || "loading"} className="settings-form" onSubmit={save}><label>Fiverr Gig URL<input name="fiverr_gig_url" type="url" required defaultValue={settings.fiverr_gig_url}/></label><div className="form-grid"><label>Your name<input name="sender_name" required defaultValue={settings.sender_name}/></label><label>Business name<input name="sender_business_name" required defaultValue={settings.sender_business_name}/></label></div><label>Postal address for country-appropriate outreach<input name="sender_postal_address" defaultValue={settings.sender_postal_address}/></label><div className="form-grid"><label>Follow-up delay (days)<input name="follow_up_delay_days" type="number" min="1" max="30" defaultValue={settings.follow_up_delay_days || "5"}/></label><label>Backups retained<input name="backup_retention_count" type="number" min="1" max="30" defaultValue={settings.backup_retention_count || "10"}/></label></div>{error && <p className="form-error">{error}</p>}<button className="primary-action" disabled={!settings.fiverr_gig_url}>Save settings</button></form></div><div className="panel"><div className="panel-heading"><h3>Local data</h3><span>No API keys included</span></div><div className="data-actions"><p>Backups and exports are written beside the portable application on D:. Spreadsheet formulas are neutralized in CSV exports.</p><button className="secondary-action" onClick={() => void backup()}>Create database backup</button><button className="secondary-action" onClick={() => void exportData("csv")}>Export leads as CSV</button><button className="secondary-action" onClick={() => void exportData("json")}>Export leads as JSON</button></div></div></section>;
}

function AddLeadDialog({ onClose, onCreated }: { onClose: () => void; onCreated: () => Promise<void> }) {
  const [saving, setSaving] = useState(false); const [error, setError] = useState<string>();
  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault(); setError(undefined);
    const data = new FormData(event.currentTarget);
    let website = String(data.get("officialWebsite") || "").trim();
    if (website && !website.includes("://")) website = `https://${website}`;
    const candidate = { companyName: data.get("companyName"), officialWebsite: website, country: data.get("country"), language: data.get("language"), cityOrServiceArea: String(data.get("cityOrServiceArea") || "") || undefined };
    const parsed = leadInputSchema.safeParse(candidate);
    if (!parsed.success) { setError(parsed.error.issues[0]?.message || "Check the lead details."); return; }
    try { setSaving(true); await addLead({ ...parsed.data, officialWebsite: normalizeOfficialWebsite(parsed.data.officialWebsite) }); await onCreated(); }
    catch (reason) { setError(typeof reason === "string" ? reason : "The lead could not be saved."); }
    finally { setSaving(false); }
  }
  return <div className="dialog-backdrop" role="presentation" onMouseDown={e => { if (e.target === e.currentTarget) onClose(); }}><section className="dialog" role="dialog" aria-modal="true" aria-labelledby="add-title"><div className="dialog-heading"><div><p className="eyebrow">Manual discovery</p><h2 id="add-title">Add official website</h2></div><button className="icon-button" onClick={onClose} aria-label="Close"><X size={18}/></button></div><form onSubmit={submit}>
    <label>Company name<input name="companyName" required minLength={2} autoFocus placeholder="Example Stays"/></label>
    <label>Official website<input name="officialWebsite" required placeholder="https://example.com" inputMode="url"/></label>
    <div className="form-grid"><label>Country<select name="country" defaultValue="GB"><option value="GB">United Kingdom</option><option value="US">United States</option><option value="FR">France</option></select></label><label>Outreach language<select name="language" defaultValue="en"><option value="en">English</option><option value="fr">French</option></select></label></div>
    <label>City or service area <span>optional</span><input name="cityOrServiceArea" placeholder="Cornwall"/></label>
    <div className="form-assurance">Adding a website creates a local record only. It does not visit or contact the company.</div>
    {error && <p className="form-error" role="alert">{error}</p>}<div className="dialog-actions"><button type="button" className="secondary-action" onClick={onClose}>Cancel</button><button className="primary-action" disabled={saving}>{saving ? "Saving…" : "Add lead"}</button></div>
  </form></section></div>;
}
