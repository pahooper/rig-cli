check:
  cargo fmt --all -- --check
  cargo clippy --all-targets --all-features -- -D warnings
  cargo test --all-features
  # cargo deny check # Uncomment when cargo-deny is installed
  # cargo audit      # Uncomment when cargo-audit is installed
  # cargo machete    # Uncomment when cargo-machete is installed
  # typos            # Uncomment when typos is installed

fmt:
  cargo fmt --all

test:
  cargo test --all-features
