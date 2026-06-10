import type { Bead } from './bead.ts';

export type GithubState = 'open' | 'closed';

/**
 * The GitHub issue state that mirrors this bead. One-directional: the sync
 * pushes this to GitHub and never reads GitHub state back into the bead.
 */
export function githubStateForBead(bead: Bead): GithubState {
  return bead.status === 'closed' ? 'closed' : 'open';
}
