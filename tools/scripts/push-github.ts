#!/usr/bin/env tsx
import { execFileSync } from 'node:child_process';
import { OctokitGithubClient } from './sync-beads-github/github.ts';
import { BdCliStore } from './sync-beads-github/beads.ts';
import { syncBeadsToGithub } from './sync-beads-github/push.ts';
import { trackLabel } from './sync-beads-github/config.ts';

function resolveRepo(): string {
  // GitHub Actions sets GITHUB_REPOSITORY="owner/name".
  if (process.env.GITHUB_REPOSITORY) return process.env.GITHUB_REPOSITORY;
  // Fallback: parse the beads sync.remote (https://github.com/owner/name.git).
  const remote = execFileSync('bd', ['config', 'get', 'sync.remote'], {
    encoding: 'utf8',
  }).trim();
  const m = /github\.com[/:]([^/]+\/[^/.]+)(?:\.git)?/.exec(remote);
  if (!m) throw new Error(`cannot resolve repo from sync.remote "${remote}"`);
  return m[1];
}

async function main(): Promise<void> {
  const dryRun = process.argv.includes('--dry-run');
  const token = process.env.GITHUB_TOKEN;
  if (!token) throw new Error('GITHUB_TOKEN is required');
  const repo = resolveRepo();

  const github = new OctokitGithubClient(token, repo);
  const beads = new BdCliStore();

  console.error(`==> Syncing beads -> GitHub (${repo})${dryRun ? ' [dry-run]' : ''}`);
  await syncBeadsToGithub({ beads, github, repo, dryRun, trackLabel: trackLabel() });

  if (!dryRun) {
    console.error('==> Capturing bd<->GitHub mapping into .beads/issues.jsonl');
    execFileSync('bd', ['export', '-o', '.beads/issues.jsonl'], { stdio: 'inherit' });
  }
  console.error('==> Sync complete');
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
