import type { Bead, ReverseBeadsStore, BeadUpdate } from './beads.ts';
import type { GithubIssue } from './github.ts';
import { refUrlForNumber, parseBody, splitLabels, issueNumberFromRef } from './transform.ts';

export interface ReverseDeps {
  beads: ReverseBeadsStore;
  repo: string; // "owner/name"
  trackLabel: string;
  logger?: (msg: string) => void;
}

/**
 * Pull one GitHub issue into its bead (the reverse of the forward mirror).
 * Find the bead by external_ref; update it, or create it if the issue carries
 * the track label; otherwise skip. The whole issue is read and applied — the
 * triggering event only identifies which issue changed.
 */
export async function reconcileIssueToBead(issue: GithubIssue, deps: ReverseDeps): Promise<void> {
  const { beads, repo, trackLabel } = deps;
  const log = deps.logger ?? console.error;

  const all = await beads.list();
  let bead: Bead | undefined = all.find(
    (b) => issueNumberFromRef(b.external_ref) === issue.number,
  );

  const { issueType, priority, status, freeform, warnings } = splitLabels(issue.labels, trackLabel);
  warnings.forEach((w) => log(`reverse-sync #${issue.number}: ${w}`));

  if (!bead) {
    if (!issue.labels.includes(trackLabel)) {
      log(`reverse-sync #${issue.number}: untracked and unlabeled; skipping`);
      return;
    }
    log(`reverse-sync: adopting GitHub #${issue.number} "${issue.title}" into beads`);
    const id = await beads.create({
      title: issue.title,
      type: issueType ?? 'task',
      externalRef: refUrlForNumber(repo, issue.number),
    });
    bead = { id, title: issue.title, status: 'open', priority: 2, issue_type: issueType ?? 'task', updated_at: '' };
  }

  const { description, design } = parseBody(issue.body);
  const fields: BeadUpdate = {
    title: issue.title,
    description,
    labels: freeform,
    owner: issue.assignees[0] ?? null,
  };
  if (design !== undefined) fields.design = design;
  if (issueType !== undefined) fields.issueType = issueType;
  if (priority !== undefined) fields.priority = priority;

  if (issue.state === 'closed') {
    await beads.update(bead.id, fields);
    if (bead.status !== 'closed') await beads.close(bead.id);
    return;
  }

  // Open issue: a non-closed status label sets (and un-closes) status in one update;
  // otherwise, if the bead was closed, reopen it.
  const statusLabel = status && status !== 'closed' ? status : undefined;
  if (statusLabel) fields.status = statusLabel;
  await beads.update(bead.id, fields);
  if (!statusLabel && bead.status === 'closed') await beads.reopen(bead.id);
}
