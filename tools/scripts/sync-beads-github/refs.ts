const ISSUE_URL_RE = /\/issues\/(\d+)\/?$/;

/** Extract a GitHub issue number from a bead external_ref (URL or bare number). */
export function issueNumberFromRef(ref: string | undefined | null): number | null {
  if (!ref) return null;
  const trimmed = ref.trim();
  const m = ISSUE_URL_RE.exec(trimmed);
  if (m) return Number(m[1]);
  const n = Number(trimmed);
  return Number.isInteger(n) && n > 0 ? n : null;
}

/** Canonical external_ref URL for a given issue number. `repo` is "owner/name". */
export function refUrlForNumber(repo: string, issueNumber: number): string {
  return `https://github.com/${repo}/issues/${issueNumber}`;
}
