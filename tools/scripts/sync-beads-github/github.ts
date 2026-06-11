import { Octokit } from '@octokit/rest';

export type GithubState = 'open' | 'closed';

export interface GithubIssue {
  number: number;
  id: number; // REST database id — required by the sub_issues API
  title: string;
  body: string | null;
  state: GithubState;
  updated_at: string;
  labels: string[];
  assignees: string[];
}

export interface CreateIssueInput {
  title: string;
  body?: string;
}

export interface UpdateIssueInput {
  title?: string;
  body?: string;
  state?: GithubState;
}

export interface GithubClient {
  listOpenIssues(): Promise<GithubIssue[]>;
  getIssue(num: number): Promise<GithubIssue | null>;
  createIssue(input: CreateIssueInput): Promise<GithubIssue>;
  updateIssue(num: number, input: UpdateIssueInput): Promise<void>;
  ensureLabelsExist(labels: string[]): Promise<void>;
  setLabels(num: number, labels: string[]): Promise<void>;
  /** Sets the assignee set; logs and continues if GitHub rejects a login. */
  setAssignees(num: number, logins: string[]): Promise<void>;
  /** Child issue numbers currently linked as sub-issues of `parentNum`. */
  listSubIssues(parentNum: number): Promise<number[]>;
  /** Links a child (by its REST database id) under a parent issue. */
  addSubIssue(parentNum: number, childId: number): Promise<void>;
}

function isPullRequest(issue: { pull_request?: unknown }): boolean {
  return issue.pull_request != null;
}

function toGithubIssue(raw: {
  number: number;
  id: number;
  title: string;
  body?: string | null;
  state: string;
  updated_at: string;
  labels: Array<string | { name?: string }>;
  assignees?: Array<{ login: string }> | null;
}): GithubIssue {
  return {
    number: raw.number,
    id: raw.id,
    title: raw.title,
    body: raw.body ?? null,
    state: raw.state === 'closed' ? 'closed' : 'open',
    updated_at: raw.updated_at,
    labels: raw.labels.map((l) => (typeof l === 'string' ? l : (l.name ?? ''))).filter(Boolean),
    assignees: (raw.assignees ?? []).map((a) => a.login),
  };
}

export class OctokitGithubClient implements GithubClient {
  private readonly octokit: Octokit;
  private readonly owner: string;
  private readonly repo: string;
  private readonly logger: (msg: string) => void;

  constructor(token: string, repo: string, logger: (msg: string) => void = console.error) {
    const [owner, name] = repo.split('/');
    if (!owner || !name) throw new Error(`invalid repo "${repo}", expected "owner/name"`);
    this.octokit = new Octokit({ auth: token });
    this.owner = owner;
    this.repo = name;
    this.logger = logger;
  }

  async listOpenIssues(): Promise<GithubIssue[]> {
    const raw = await this.octokit.paginate(this.octokit.issues.listForRepo, {
      owner: this.owner,
      repo: this.repo,
      state: 'open',
      per_page: 100,
    });
    return raw.filter((i) => !isPullRequest(i)).map(toGithubIssue);
  }

  async getIssue(num: number): Promise<GithubIssue | null> {
    try {
      const { data } = await this.octokit.issues.get({
        owner: this.owner,
        repo: this.repo,
        issue_number: num,
      });
      return toGithubIssue(data);
    } catch (err: unknown) {
      if ((err as { status?: number }).status === 404) return null;
      throw err;
    }
  }

  async createIssue(input: CreateIssueInput): Promise<GithubIssue> {
    const { data } = await this.octokit.issues.create({
      owner: this.owner,
      repo: this.repo,
      title: input.title,
      body: input.body,
    });
    return toGithubIssue(data);
  }

  async updateIssue(num: number, input: UpdateIssueInput): Promise<void> {
    await this.octokit.issues.update({
      owner: this.owner,
      repo: this.repo,
      issue_number: num,
      ...input,
    });
  }

  async ensureLabelsExist(labels: string[]): Promise<void> {
    for (const name of labels) {
      try {
        await this.octokit.issues.getLabel({ owner: this.owner, repo: this.repo, name });
      } catch (err: unknown) {
        if ((err as { status?: number }).status === 404) {
          await this.octokit.issues.createLabel({
            owner: this.owner,
            repo: this.repo,
            name,
            color: 'ededed',
          });
        } else {
          throw err;
        }
      }
    }
  }

  async setLabels(num: number, labels: string[]): Promise<void> {
    await this.octokit.issues.setLabels({
      owner: this.owner,
      repo: this.repo,
      issue_number: num,
      labels,
    });
  }

  async setAssignees(num: number, logins: string[]): Promise<void> {
    try {
      // Use the dedicated assignees endpoints (add/remove) — passing `assignees`
      // to the issue PATCH endpoint is deprecated by GitHub. Diff against the
      // current set so we end up with exactly `logins`.
      const issue = await this.getIssue(num);
      const current = issue?.assignees ?? [];
      const toRemove = current.filter((l) => !logins.includes(l));
      const toAdd = logins.filter((l) => !current.includes(l));
      if (toRemove.length > 0) {
        await this.octokit.issues.removeAssignees({
          owner: this.owner,
          repo: this.repo,
          issue_number: num,
          assignees: toRemove,
        });
      }
      if (toAdd.length > 0) {
        await this.octokit.issues.addAssignees({
          owner: this.owner,
          repo: this.repo,
          issue_number: num,
          assignees: toAdd,
        });
      }
    } catch (err: unknown) {
      this.logger(
        `warn: could not set assignees ${JSON.stringify(logins)} on #${num}: ${
          (err as Error).message
        } (see assignee-map seam in push.ts)`,
      );
    }
  }

  async listSubIssues(parentNum: number): Promise<number[]> {
    try {
      const data = await this.octokit.paginate(
        'GET /repos/{owner}/{repo}/issues/{issue_number}/sub_issues',
        { owner: this.owner, repo: this.repo, issue_number: parentNum, per_page: 100 },
      );
      return (data as Array<{ number: number }>).map((i) => i.number);
    } catch (err: unknown) {
      if ((err as { status?: number }).status === 404) return [];
      throw err;
    }
  }

  async addSubIssue(parentNum: number, childId: number): Promise<void> {
    await this.octokit.request(
      'POST /repos/{owner}/{repo}/issues/{issue_number}/sub_issues',
      {
        owner: this.owner,
        repo: this.repo,
        issue_number: parentNum,
        sub_issue_id: childId,
      },
    );
  }
}
