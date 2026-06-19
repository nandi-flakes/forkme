# forkme

A tool for managing forks of large projects using a patch-based approach.

## The Problem

When maintaining a fork of a large project, there are two common approaches:

- **Rebase**: Rewrite your changes on top of upstream. Clean history, but requires force-pushing.
- **Merge**: Merge upstream into your branch. Preserves history, but creates merge commits that may not build cleanly, making bisecting difficult.

## The Solution

**forkme** takes a different approach: keep your changes as a set of patches, organized by file path. This gives you:

- Clean history of your own changes
- Simple conflict resolution during upstream updates
- No force-pushes required

## Project Structure

A forkme-managed project looks like this:

```
my-fork/
├── forkme.toml     # Configuration (upstream URL, branch)
├── patches/        # Your patches, organized by file path
│   ├── src/
│   │   └── main.rs.patch
│   └── Cargo.toml.patch
└── source/         # Upstream repo with your changes (in .gitignore)
```

Only `forkme.toml` and `patches/` are committed to your repository.

## Installation

```bash
cargo install --path .
```

## Development

If you use `devenv`, enter the project shell with:

```bash
devenv shell
```

This provides Rust and Git. It also exposes a few helper commands inside the shell:

```bash
cargo-test
cargo-fmt
cargo-lint
```

If you use `direnv`, allow the checked-in `.envrc` once:

```bash
direnv allow
```

## Usage

### Initialize a new project

```bash
forkme init --url https://github.com/user/repo --branch main
```

This will:
- Create `forkme.toml` with upstream configuration
- Clone the upstream repo into `source/`
- Create a `forkme` branch for your work
- Set up `.gitignore` to exclude `source/`

### Make changes

Work in the `source/` directory on the `forkme` branch:

```bash
cd source
# edit files, make commits
git commit -m "My changes"
```

### Sync changes to patches

```bash
forkme sync
```

This updates patch files in `patches/` based on your changes.

### Update from upstream

```bash
forkme update
```

This fetches upstream and rebases your `forkme` branch. If there are conflicts, resolve them with standard git commands, then run `forkme sync` to regenerate patches.

### Apply patches (after fresh clone)

```bash
forkme init  # Uses existing forkme.toml
```

Or to reset and reapply:

```bash
forkme apply
```

### Check status

```bash
forkme status  # Project overview
forkme stats   # Patch statistics (added/modified/deleted)
```

## Commands

| Command | Description |
|---------|-------------|
| `init --url <url> [--branch <branch>]` | Initialize project with upstream repo |
| `apply` | Reset to upstream and reapply all patches |
| `sync` | Generate patches from current changes |
| `update` | Fetch upstream and rebase |
| `status` | Show project status |
| `stats` | Show patch statistics |

## License

AGPL 3.0
