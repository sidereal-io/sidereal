#!/usr/bin/env tsx
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { OctokitGithubClient } from './sync-beads-github/github.ts';
import { BdCliStore } from './sync-beads-github/beads.ts';
import { reconcileIssueToBead } from './sync-beads-github/pull.ts';
import { trackLabel, botLogin } from './sync-beads-github/config.ts';

interface IssueEvent {
  action?: string;
  issue?: { number: number };
  sender?: { login: string };
}

function readEvent(): IssueEvent {
  const path = process.env.GITHUB_EVENT_PATH;
  if (!path) throw new Error('GITHUB_EVENT_PATH is not set');
  return JSON.parse(readFileSync(path, 'utf8')) as IssueEvent;
}

function resolveRepo(): string {
  if (process.env.GITHUB_REPOSITORY) return process.env.GITHUB_REPOSITORY;
  const remote = execFileSync('bd', ['config', 'get', 'sync.remote'], { encoding: 'utf8' }).trim();
  const m = /github\.com[/:]([^/]+\/[^/.]+)(?:\.git)?/.exec(remote);
  if (!m) throw new Error(`cannot resolve repo from sync.remote "${remote}"`);
  return m[1];
}

async function main(): Promise<void> {
  const token = process.env.GITHUB_TOKEN;
  if (!token) throw new Error('GITHUB_TOKEN is required');
  const event = readEvent();

  if (event.action === 'deleted') {
    console.error(`reverse-sync: issue ${event.issue?.number} deleted on GitHub — leaving bead intact`);
    return;
  }
  if (event.sender?.login === botLogin()) {
    console.error('reverse-sync: event from our own bot — skipping (loop guard)');
    return;
  }
  if (!event.issue?.number) {
    console.error('reverse-sync: no issue in event payload — skipping');
    return;
  }

  const repo = resolveRepo();
  const github = new OctokitGithubClient(token, repo);
  const issue = await github.getIssue(event.issue.number);
  if (!issue) {
    console.error(`reverse-sync: issue #${event.issue.number} not found — skipping`);
    return;
  }

  await reconcileIssueToBead(issue, { beads: new BdCliStore(), repo, trackLabel: trackLabel() });

  console.error('reverse-sync: pushing beads state to Dolt');
  execFileSync('bd', ['dolt', 'push'], { stdio: 'inherit' });
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
