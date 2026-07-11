# Contributing

The branching strategy, commit convention, and release process for this repo follow the canonical rules documented in my engineering handbook:

- **Why:** [Branching Strategy philosophy](https://github.com/amcheste/engineering-handbook/blob/main/docs/philosophies/branching-strategy.md)
- **How:** [Branching & Releases workflow](https://github.com/amcheste/engineering-handbook/blob/main/docs/workflows/branching-and-releases.md)

In short:

- `main` = latest release. `develop` = integration branch (default).
- Branch from `develop`. One logical change per PR. PRs target `develop`.
- [Conventional Commits](https://www.conventionalcommits.org/) — `feat:` / `fix:` / `docs:` / `chore:` / `refactor:`, with `!` for breaking changes.
- Releases are cut by `/publish-release` — version bump PR to `develop`, then `develop → main` via CLI merge (never GitHub's merge button), then semver tag.

Repo-specific contribution notes (if any) live below this line.

---

<!-- Project-specific contributing notes go here. Keep this section small — if a note applies to all repos, it belongs in the handbook, not here. -->
