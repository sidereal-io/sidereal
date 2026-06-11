/** The label that marks a GitHub issue as tracked in beads (opt-in intake). */
export function trackLabel(): string {
  return process.env.BEADS_TRACK_LABEL ?? 'beads';
}

/** The login whose issue events are our own forward writes (skip to break loops). */
export function botLogin(): string {
  return process.env.BEADS_SYNC_BOT ?? 'github-actions[bot]';
}
