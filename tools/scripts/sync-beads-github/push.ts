import type { Bead } from './bead.ts';
import { blockerBeadIds, parentBeadId } from './bead.ts';
import type { BeadsStore } from './beads-store.ts';
import type { GithubClient } from './github.ts';
import {
  renderBody,
  extractMarkerId,
  computeManagedLabels,
  reconcileLabels,
  refUrlForNumber,
  githubStateForBead,
} from './to-github.ts';
import { issueNumberFromRef } from './from-github.ts';

export interface SyncOptions {
  beads: BeadsStore;
  github: GithubClient;
  repo: string; // "owner/name"
  dryRun: boolean;
  trackLabel: string;
  logger?: (msg: string) => void;
}

/**
 * Map a bead's owner (assignee) to GitHub logins. Pass-through verbatim today.
 * SEAM: to add a bd-name -> GitHub-login table, edit this function.
 */
function ownerToLogins(owner: string | undefined): string[] {
  return owner ? [owner] : [];
}

export async function syncBeadsToGithub(opts: SyncOptions): Promise<void> {
  const { github, beads, repo, dryRun, trackLabel } = opts;
  const log = opts.logger ?? console.error;
  const ref = (n: number) => refUrlForNumber(repo, n);

  // --- Intake: adopt human-created GitHub issues into beads (once) ---
  let beadList = await beads.list();
  const referencedNumbers = new Set(
    beadList
      .map((b) => issueNumberFromRef(b.external_ref))
      .filter((n): n is number => n != null),
  );
  for (const issue of await github.listOpenIssues()) {
    if (referencedNumbers.has(issue.number)) continue;
    if (extractMarkerId(issue.body)) continue; // ours, but bead lost its ref — don't duplicate
    if (!issue.labels.includes(trackLabel)) continue; // opt-in: only adopt labeled issues
    log(`intake: adopting GitHub #${issue.number} "${issue.title}" into beads`);
    if (!dryRun) {
      await beads.create({
        title: issue.title,
        description: issue.body ?? undefined,
        type: 'task',
        externalRef: ref(issue.number),
      });
    }
  }
  if (!dryRun) beadList = await beads.list();

  // Gates are internal workflow artifacts — never mirror them to GitHub, and
  // never let a gate count as a blocker on a real issue (see Task 11/12).
  const gateIds = new Set(beadList.filter((b) => b.issue_type === 'gate').map((b) => b.id));
  const syncable = beadList.filter((b) => b.issue_type !== 'gate');
  const realBlockerIds = (bead: Bead) => blockerBeadIds(bead).filter((id) => !gateIds.has(id));

  // --- Phase 1: ensure every (non-gate) bead maps to a GitHub issue number ---
  const numberByBead = new Map<string, number>();
  for (const bead of syncable) {
    let num = issueNumberFromRef(bead.external_ref);
    if (num == null) {
      // Don't resurrect history: a bead that's already closed and was never
      // mirrored gets no GitHub issue.
      if (githubStateForBead(bead) === 'closed') continue;
      log(`create: GitHub issue for bead ${bead.id} "${bead.title}"`);
      if (dryRun) continue;
      const created = await github.createIssue({ title: bead.title });
      num = created.number;
      await beads.setExternalRef(bead.id, ref(num));
    }
    numberByBead.set(bead.id, num);
  }
  if (dryRun) {
    log('dry run: skipping content + relationship sync');
    return;
  }

  // status lookup for blocker open/closed checks
  const statusByBead = new Map(beadList.map((b) => [b.id, b.status]));

  // --- Phase 2: full content sync (body, labels, assignee, state) ---
  for (const bead of syncable) {
    const num = numberByBead.get(bead.id);
    if (num == null) continue;
    const issue = await github.getIssue(num);
    if (!issue) {
      log(`warn: bead ${bead.id} references missing issue #${num}; skipping`);
      continue;
    }

    const blockerNums = realBlockerIds(bead)
      .map((id) => numberByBead.get(id))
      .filter((n): n is number => n != null);
    const hasOpenBlocker = realBlockerIds(bead).some(
      (id) => (statusByBead.get(id) ?? 'open') !== 'closed',
    );

    const body = renderBody(bead, blockerNums);
    const desiredLabels = reconcileLabels(
      issue.labels,
      [...computeManagedLabels(bead, hasOpenBlocker), trackLabel],
      bead.labels ?? [],
    );
    await github.ensureLabelsExist(desiredLabels);
    await github.setLabels(num, desiredLabels);
    await github.setAssignees(num, ownerToLogins(bead.owner));

    // One-directional state mirror: push the bead's state to GitHub; never
    // read GitHub state back into the bead, and never close a bead here.
    const desiredState = githubStateForBead(bead);
    await github.updateIssue(num, {
      title: bead.title,
      body,
      ...(issue.state !== desiredState ? { state: desiredState } : {}),
    });
  }

  // --- Phase 3: native sub-issue links ---
  for (const bead of syncable) {
    const parentId = parentBeadId(bead);
    if (!parentId || gateIds.has(parentId)) continue;
    const parentNum = numberByBead.get(parentId);
    const childNum = numberByBead.get(bead.id);
    if (parentNum == null || childNum == null) continue;

    const existing = await github.listSubIssues(parentNum);
    if (existing.includes(childNum)) continue;
    const childIssue = await github.getIssue(childNum);
    if (!childIssue) continue;
    log(`sub-issue: link #${childNum} under #${parentNum}`);
    await github.addSubIssue(parentNum, childIssue.id);
  }
}
