# Velo Hub

## Release workflow

This repo uses [Changesets](https://github.com/changesets/changesets) with conventional commit messages to gate every production release.

1. Run `bun run changeset` on your feature branch and follow the prompts to describe the change and choose the semantic version bump.
2. Commit the generated `.changeset/*.md` file along with your code and open a pull request.
3. When the PR lands on `main`, the `Release` GitHub Action installs dependencies, runs lint/build, versions the repo via `changeset version`, commits the change (`chore(release): publish`), tags it, creates a GitHub Release, and then builds/signs the Tauri desktop bundles for macOS, Windows, and Linux via a matrix job. All generated installers are attached to the release automatically.

Release commits and tags are fully automated, so you do not need to bump versions manually. If a change does not require a release, create an empty changeset with `bun run changeset --empty`.
