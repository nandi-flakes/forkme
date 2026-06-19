{ pkgs, ... }:
{
  packages = with pkgs; [
    git
  ];

  languages.rust.enable = true;

  enterShell = ''
    echo "forkme development shell"
    rustc --version
    cargo --version
    git --version
  '';

  scripts = {
    cargo-test.exec = "cargo test";
    cargo-fmt.exec = "cargo fmt --all";
    cargo-lint.exec = "cargo fmt --all -- --check && cargo clippy --all-targets --all-features -- -D warnings";
  };
}
