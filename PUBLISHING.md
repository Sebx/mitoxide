# Publishing Mitoxide to Crates.io

This guide walks you through publishing the Mitoxide crates to crates.io.

## Prerequisites

1. **Crates.io Account**: Create an account at [crates.io](https://crates.io)
2. **API Token**: Get your API token from [crates.io/me](https://crates.io/me)
3. **Login**: Run `cargo login <your-token>` to authenticate

## Pre-Publishing Checklist

- [ ] All crates build successfully: `cargo build --workspace --release`
- [ ] All tests pass: `cargo test --workspace`
- [ ] Documentation builds: `cargo doc --workspace --no-deps`
- [ ] Version numbers are correct in all `Cargo.toml` files
- [ ] README files exist for all crates
- [ ] License is set correctly (MIT)
- [ ] Repository URLs are updated

## Publishing Order

Crates must be published in dependency order:

1. **mitoxide-proto** (no dependencies)
2. **mitoxide-wasm** (no dependencies)
3. **mitoxide-ssh** (depends on mitoxide-proto)
4. **mitoxide-agent** (depends on mitoxide-proto, mitoxide-wasm)
5. **mitoxide** (depends on all above)

## Manual Publishing Steps

### 1. Publish mitoxide-proto

```bash
cd crates/mitoxide-proto
cargo publish --dry-run  # Check for issues
cargo publish            # Actual publish
```

### 2. Publish mitoxide-wasm

```bash
cd crates/mitoxide-wasm
cargo publish --dry-run
cargo publish
```

### 3. Publish mitoxide-ssh

```bash
cd crates/mitoxide-ssh
cargo publish --dry-run
cargo publish
```

### 4. Publish mitoxide-agent

```bash
cd crates/mitoxide-agent
cargo publish --dry-run
cargo publish
```

### 5. Publish mitoxide (main crate)

```bash
cd crates/mitoxide
cargo publish --dry-run
cargo publish
```

## Using the Publishing Script

Alternatively, use the provided script:

### Windows (PowerShell)
```powershell
.\scripts\publish.ps1
```

### Linux/macOS
```bash
./scripts/publish.sh
```

## Post-Publishing Steps

1. **Verify on Crates.io**: Check that all crates appear at `https://crates.io/crates/CRATE_NAME`
2. **Check Documentation**: Verify docs build at `https://docs.rs/CRATE_NAME`
3. **Update Repository**: 
   - Create a git tag: `git tag v0.1.0 && git push origin v0.1.0`
   - Create a GitHub release with release notes
4. **Update Dependencies**: In future versions, update path dependencies to use published versions

## Troubleshooting

### Common Issues

1. **"crate already exists"**: Version already published, increment version number
2. **"authentication failed"**: Run `cargo login` with your API token
3. **"dependency not found"**: Wait a few minutes for crates.io to index dependencies
4. **"documentation failed to build"**: Check for missing documentation or broken links

### Dry Run Failures

Always run `cargo publish --dry-run` first to catch issues:
- Missing README files
- Invalid metadata
- Dependency issues
- File inclusion problems

### Version Management

For future releases:
1. Update version numbers in all `Cargo.toml` files
2. Update `CHANGELOG.md`
3. Update inter-crate dependencies to use new versions
4. Follow semantic versioning (semver)

## Yanking Versions

If you need to yank a version:

```bash
cargo yank --vers 0.1.0 mitoxide-proto
```

To un-yank:

```bash
cargo yank --vers 0.1.0 --undo mitoxide-proto
```

## Automation

The GitHub Actions workflow in `.github/workflows/ci.yml` includes automated publishing on tagged releases. To use it:

1. Set `CRATES_IO_TOKEN` secret in GitHub repository settings
2. Create and push a version tag: `git tag v0.1.0 && git push origin v0.1.0`
3. The workflow will automatically publish all crates

## Support

If you encounter issues:
- Check the [Cargo Book](https://doc.rust-lang.org/cargo/reference/publishing.html)
- Ask on [users.rust-lang.org](https://users.rust-lang.org)
- Check crates.io status at [status.crates.io](https://status.crates.io)