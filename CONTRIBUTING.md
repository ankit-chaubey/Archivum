# Contributing to Archivum

Thank you for your interest in contributing! Archivum is an open-source project and all contributions are welcome.

---

## Getting Started

```bash
git clone https://github.com/ankit-chaubey/Archivum
cd Archivum
cargo build
cargo test
```

### Prerequisites

- Rust 1.75+
- `cargo clippy`
- `cargo fmt`

---

## How to Contribute

### Reporting Bugs

Open an issue using the **Bug Report** template. Please include:
- Your OS and Rust version (`rustc --version`)
- The exact command you ran
- The full error output
- Expected vs actual behaviour

### Requesting Features

Open an issue using the **Feature Request** template. Describe the use case — not just the solution.

### Submitting a PR

1. Fork the repository
2. Create a feature branch: `git checkout -b feat/my-feature`
3. Make your changes
4. Ensure all checks pass:
   ```bash
   cargo fmt --all
   cargo clippy --all-targets -- -D warnings
   cargo test --all
   ```
5. Open a PR with a clear description of what and why

---

## Code Style

- Follow `rustfmt` formatting (enforced in CI)
- No `clippy` warnings (enforced in CI)
- New commands must have a module in `src/` and a corresponding entry in `src/main.rs`
- All public functions should have doc comments
- Add Apache 2.0 license header to new source files

---

## Commit Messages

Use conventional commits:
```
feat: add --zstd-level flag to create command
fix: skip symlinks in md5 roundtrip comparison
docs: update search command usage in README
chore: bump clap to 4.5.8
```

---

## Testing

Run the full test suite:
```bash
cargo test --all
```

For integration tests (requires a built binary):
```bash
cargo build --release
bash test_all.sh
```

---

## License

By contributing, you agree your contributions will be licensed under Apache 2.0.

---

*Made with ♥ by [Ankit Chaubey](https://github.com/ankit-chaubey)*
