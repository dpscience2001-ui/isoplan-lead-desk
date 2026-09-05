export function isBusinessEmail(value: string): boolean {
  if (value.length > 254 || /\s/.test(value)) return false;
  const [local, domain, extra] = value.split("@");
  return !extra && Boolean(local && local.length <= 64 && domain?.includes(".") && !domain.startsWith(".") && !domain.endsWith("."));
}

export function gmailComposeUrl(recipient: string, subject: string, body: string): string {
  if (!isBusinessEmail(recipient)) throw new Error("A valid recipient email is required.");
  if (!subject.trim() || !body.trim()) throw new Error("Subject and body are required.");
  const query = new URLSearchParams({ view: "cm", fs: "1", to: recipient, su: subject, body });
  return `https://mail.google.com/mail/?${query.toString()}`;
}

