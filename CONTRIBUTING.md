# Contributing to Mitoxide 🤝

Thank you for your interest in contributing to Mitoxide! We welcome contributions from developers of all skill levels and backgrounds. This guide will help you get started.

## 🌟 Ways to Contribute

### 🐛 Bug Reports
- Search existing issues before creating new ones
- Use the bug report template
- Include reproduction steps, system info, and error logs
- Add relevant labels and screenshots

### 💡 Feature Requests
- Start with a discussion to gather feedback
- Use the feature request template
- Explain the use case and expected behavior
- Consider implementation complexity

### 🔧 Code Contributions
- Fork the repository
- Create a feature branch with a descriptive name
- Write tests for new functionality
- Follow the coding standards
- Submit a pull request

### 📝 Documentation
- Fix typos and improve clarity
- Add examples and use cases
- Update API documentation
- Translate content

### 🎨 Design & UX
- Improve error messages
- Design better APIs
- Create visual documentation
- Enhance user experience

## 🚀 Getting Started

### Prerequisites
- Rust 1.82+
- Git
- Docker (for integration tests)
- SSH client

### Development Setup

1. **Fork and Clone**
   ```bash
   git clone https://github.com/yourusername/mitoxide.git
   cd mitoxide
   ```

2. **Install Dependencies**
   ```bash
   # Install Rust if needed
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   
   # Install development tools
   cargo install cargo-watch cargo-tarpaulin
   ```

3. **Build and Test**
   ```bash
   # Build the workspace
   cargo build --workspace
   
   # Run unit tests
   cargo test --workspace
   
   # Run integration tests (requires Docker)
   ./scripts/test_routing.sh
   ```

4. **Set up Pre-commit Hooks**
   ```bash
   # Install pre-commit
   pip install pre-commit
   
   # Install hooks
   pre-commit install
   ```

## 📋 Development Guidelines

### Code Style
- Follow Rust standard formatting (`cargo fmt`)
- Use `cargo clippy` for linting
- Write clear, self-documenting code
- Add comments for complex logic

### Testing
- Write unit tests for all new functionality
- Add integration tests for end-to-end scenarios
- Maintain test coverage above 80%
- Use descriptive test names

### Documentation
- Document all public APIs with rustdoc
- Include examples in documentation
- Update README for significant changes
- Write clear commit messages

### Performance
- Profile performance-critical code
- Add benchmarks for key operations
- Consider memory usage and allocations
- Test with realistic workloads

## 🔄 Pull Request Process

### Before Submitting
1. **Sync with upstream**
   ```bash
   git fetch upstream
   git rebase upstream/main
   ```

2. **Run checks**
   ```bash
   cargo fmt --check
   cargo clippy -- -D warnings
   cargo test --workspace
   ```

3. **Update documentation**
   - Update CHANGELOG.md
   - Add/update examples
   - Update API documentation

### PR Guidelines
- Use a clear, descriptive title
- Fill out the PR template completely
- Link related issues
- Request reviews from relevant maintainers
- Be responsive to feedback

### Review Process
1. Automated checks must pass
2. At least one maintainer approval required
3. All conversations must be resolved
4. Squash commits before merging

## 🏗️ Project Structure

```
mitoxide/
├── crates/
│   ├── mitoxide/          # Main SDK
│   ├── mitoxide-agent/    # Remote agent
│   ├── mitoxide-proto/    # Protocol definitions
│   ├── mitoxide-ssh/      # SSH transport
│   └── mitoxide-wasm/     # WASM runtime
├── examples/              # Usage examples
├── tests/                 # Integration tests
├── docs/                  # Documentation
├── scripts/               # Build and test scripts
└── .github/               # GitHub workflows
```

## 🧪 Testing Strategy

### Unit Tests
- Test individual functions and modules
- Mock external dependencies
- Focus on edge cases and error conditions

### Integration Tests
- Test complete workflows
- Use Docker containers for realistic environments
- Test different platforms and configurations

### Performance Tests
- Benchmark critical operations
- Test under load
- Monitor resource usage

## 📚 Resources

### Learning Rust
- [The Rust Book](https://doc.rust-lang.org/book/)
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)
- [Rustlings](https://github.com/rust-lang/rustlings)

### Project-Specific
- [Architecture Overview](docs/ARCHITECTURE.md)
- [API Design Guidelines](docs/API_DESIGN.md)
- [Performance Guide](docs/PERFORMANCE.md)

### Tools
- [Rust Analyzer](https://rust-analyzer.github.io/) - IDE support
- [cargo-watch](https://github.com/watchexec/cargo-watch) - Auto-rebuild
- [cargo-tarpaulin](https://github.com/xd009642/tarpaulin) - Code coverage

## 🎯 Good First Issues

Looking for a place to start? Check out issues labeled:
- `good first issue` - Perfect for newcomers
- `help wanted` - Community contributions welcome
- `documentation` - Improve docs and examples
- `testing` - Add tests and improve coverage

## 🤔 Questions?

- 💬 [Discord Server](https://discord.gg/YOUR_INVITE) - Real-time help
- 🐛 [GitHub Issues](https://github.com/yourusername/mitoxide/issues) - Bug reports
- 💡 [GitHub Discussions](https://github.com/yourusername/mitoxide/discussions) - Ideas and questions
- 📧 Email: maintainers@mitoxide.dev

## 🏆 Recognition

Contributors are recognized in:
- README.md contributors section
- Release notes
- Annual contributor highlights
- Special Discord roles

## 📜 Code of Conduct

We are committed to providing a welcoming and inclusive environment. Please read our [Code of Conduct](CODE_OF_CONDUCT.md) before participating.

## 📄 License

By contributing to Mitoxide, you agree that your contributions will be licensed under the MIT License.

---

**Thank you for helping make Mitoxide better! 🚀**