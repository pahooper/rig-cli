check:
  cargo fmt --all -- --check
  cargo clippy --all-targets --all-features -- -D warnings
  cargo test --all-features
  cargo audit

fmt:
  cargo fmt --all

test:
  cargo test --all-features

# Scan dependencies for known vulnerabilities (RustSec Advisory Database)
audit:
  cargo audit

# Update the advisory database and scan
audit-update:
  cargo audit fetch && cargo audit

# Show outdated root dependencies
outdated:
  cargo outdated --root-deps-only
