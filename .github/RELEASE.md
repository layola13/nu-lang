# Release Guide

## Automated Release Process

This project uses GitHub Actions to automatically build and release binaries for multiple platforms.

## Creating a New Release

### Method 1: Tag-based Release (Recommended)

When you push a tag starting with `v`, the release workflow will automatically:
1. Read the version from `Cargo.toml`
2. Build binaries for all platforms
3. Create a GitHub release
4. Upload all binaries and checksums

**Steps:**

```bash
# Make sure Cargo.toml version is updated
# Current version: 1.3.1

# Create and push a tag
git tag v1.3.1
git push origin v1.3.1

# Or create a tag with a message
git tag -a v1.3.1 -m "Release version 1.3.1"
git push origin v1.3.1
```

### Method 2: Manual Trigger

You can also manually trigger the release workflow from the GitHub Actions tab:

1. Go to the "Actions" tab in your repository
2. Select "Release" workflow
3. Click "Run workflow"
4. Select the branch and click "Run workflow"

## Version Management

The version number is automatically read from `Cargo.toml`:

```toml
[package]
version = "1.3.1"
```

**Before creating a release:**
1. Update the version in `Cargo.toml`
2. Commit the change
3. Create and push the tag matching the version

## Release Artifacts

Each release includes binaries for:

### Linux
- `nu-compiler-linux-x86_64.tar.gz` - x86_64 (Intel/AMD 64-bit)
- `nu-compiler-linux-aarch64.tar.gz` - ARM64 (ARM 64-bit)

### macOS
- `nu-compiler-macos-x86_64.tar.gz` - Intel Mac
- `nu-compiler-macos-aarch64.tar.gz` - Apple Silicon (M1/M2/M3)

### Windows
- `nu-compiler-windows-x86_64.tar.gz` - 64-bit Windows

Each archive includes:
- `rust2nu` - Rust to Nu converter
- `nu2rust` - Nu to Rust converter
- `cargo2nu` - Cargo project converter
- `nu2cargo` - Nu project converter
- `README.md` - English documentation
- `ReadMeCN.md` - Chinese documentation

### Checksums

Each `.tar.gz` file has a corresponding `.sha256` file for verification:

```bash
# Verify download (Linux/macOS)
shasum -a 256 -c nu-compiler-linux-x86_64.tar.gz.sha256

# Verify download (Windows PowerShell)
Get-FileHash nu-compiler-windows-x86_64.tar.gz -Algorithm SHA256
```

## Continuous Integration

The CI workflow runs on every push and pull request to `main`, `master`, or `develop` branches:

- Runs tests on Linux, macOS, and Windows
- Checks code formatting with `cargo fmt`
- Runs linter with `cargo clippy`
- Builds all four binaries
- Tests example conversions

## Troubleshooting

### Release workflow fails

1. Check the Actions tab for error logs
2. Verify `Cargo.toml` syntax is correct
3. Ensure all binaries build locally: `cargo build --release`
4. Check that the tag format is correct (starts with `v`)

### Missing binaries in release

The workflow builds 4 binaries for each platform. If any are missing:
1. Check the build logs in GitHub Actions
2. Verify the binary names in `Cargo.toml`
3. Ensure all source files exist in `src/bin/`

### Version mismatch

The tag version should match `Cargo.toml` version:
- Tag: `v1.3.1`
- Cargo.toml: `version = "1.3.1"`

If they don't match, delete the tag and recreate it:

```bash
# Delete local tag
git tag -d v1.3.1

# Delete remote tag
git push origin :refs/tags/v1.3.1

# Create new tag with correct version
git tag v1.3.1
git push origin v1.3.1
```

## Release Checklist

- [ ] Update version in `Cargo.toml`
- [ ] Update CHANGELOG or release notes if applicable
- [ ] Commit all changes
- [ ] Create git tag matching version: `git tag v1.3.1`
- [ ] Push tag: `git push origin v1.3.1`
- [ ] Wait for GitHub Actions to complete
- [ ] Verify release on GitHub Releases page
- [ ] Test download and installation of binaries
- [ ] Update documentation if needed