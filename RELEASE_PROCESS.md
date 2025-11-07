# Release Process

This document describes how to release a new version of `try-cli` to GitHub and crates.io.

## Prerequisites

### One-time setup for crates.io publishing

The automated release workflow publishes to crates.io. This requires:

1. **Create a crates.io account** (if you don't have one):
   - Visit https://crates.io
   - Sign in with your GitHub account

2. **Generate an API token**:
   - Go to https://crates.io/settings/tokens
   - Click "New Token"
   - Give it a name (e.g., "try-cli GitHub Actions")
   - Select the "publish-update" scope
   - Copy the generated token

3. **Add token to GitHub repository secrets**:
   - Go to your repository on GitHub
   - Navigate to Settings → Secrets and variables → Actions
   - Click "New repository secret"
   - Name: `CARGO_REGISTRY_TOKEN`
   - Value: paste the token from step 2
   - Click "Add secret"

## Release Checklist

### 1. Prepare the release

- [ ] Ensure all desired changes are merged to the main branch
- [ ] Update version in `Cargo.toml` (e.g., `0.1.0` → `0.1.1`)
- [ ] Run tests locally: `cargo test --all --locked`
- [ ] Run linting: `cargo clippy --all-targets -- -D warnings`
- [ ] Verify package contents: `cargo package --list`
- [ ] Test publish (dry-run): `cargo publish --dry-run`
- [ ] Update CHANGELOG.md (if you maintain one) with release notes
- [ ] Commit version bump: `git commit -am "chore: bump version to X.Y.Z"`
- [ ] Push to main: `git push origin main`

### 2. Create and push the release tag

```bash
# Create a new version tag (example for version 0.1.0)
git tag v0.1.0

# Push the tag to trigger the release workflow
git push origin v0.1.0
```

### 3. Monitor the release

1. Go to the **Actions** tab on GitHub
2. Watch the "Release" workflow run
3. The workflow will:
   - Create a GitHub release with binaries for all platforms
   - Run `cargo publish --dry-run` to verify
   - Publish the package to crates.io

### 4. Verify the release

After the workflow completes successfully:

- [ ] Check the [GitHub releases page](https://github.com/dariuszparys/try-rs/releases) for the new release
- [ ] Verify binaries are attached to the release
- [ ] Check [crates.io](https://crates.io/crates/try-cli) for the new version
- [ ] Test installation: `cargo install try-cli`
- [ ] Verify the installed binary works: `try --help`

## Version Numbering

This project follows [Semantic Versioning](https://semver.org/):

- **Major** (X.0.0): Breaking changes / incompatible API changes
- **Minor** (0.X.0): New features, backward-compatible
- **Patch** (0.0.X): Bug fixes, backward-compatible

During pre-1.0 development:
- Breaking changes can bump the minor version
- The API is not yet stable

## Troubleshooting

### Release workflow fails at crates.io publish

**Common causes:**

1. **Token not set or expired**
   - Verify `CARGO_REGISTRY_TOKEN` is set in GitHub secrets
   - Generate a new token if needed

2. **Version already published**
   - Crates.io does not allow re-publishing the same version
   - Bump the version and create a new tag

3. **Package name conflict**
   - The package name `try-cli` should be available
   - If taken, update `Cargo.toml` with a different name

4. **Missing required metadata**
   - Ensure `Cargo.toml` has: name, version, license, description, repository

### Manual publish

If automated publishing fails, you can publish manually:

```bash
# Login to crates.io (one-time, stores token locally)
cargo login

# Publish the current version
cargo publish
```

### Rolling back a release

**Note:** You cannot unpublish a version from crates.io after 24 hours.

- To mark a version as yanked (discourages new usage but doesn't break existing):
  ```bash
  cargo yank --vers 0.1.0 try-cli
  ```

- To un-yank:
  ```bash
  cargo yank --undo --vers 0.1.0 try-cli
  ```

## Notes

- **Binary name vs package name**: The package is called `try-cli` on crates.io, but the binary is named `try`. Users install with `cargo install try-cli` but use the `try` command.

- **First release**: For the first release (0.1.0), consider doing a manual test publish before enabling automated releases.

- **GitHub binary releases**: These are independent of crates.io and will be created even if crates.io publishing fails (they run in separate workflow jobs).
