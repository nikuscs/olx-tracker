---
name: commit
description: Create a git commit following project conventions. Use when the user asks to commit changes or create a commit.
disable-model-invocation: true
allowed-tools: Bash(git *)
---

# Create Git Commit

Follow the project's git commit conventions to create a well-formed commit.

## Workflow

1. **Check status and diff**:

```bash
git status
git diff --stat
git diff
git log --oneline -5  # See recent commit style
```

1. **Analyze changes** and draft a commit message:
   - Identify the nature of changes (feature, fix, refactor, test, docs, chore)
   - Use appropriate prefix: `feat:`, `fix:`, `test:`, `docs:`, `refactor:`, `chore:`
   - Write concise summary (50-72 chars)
   - Add detailed body if needed

2. **Stage and commit**:

```bash
# Stage specific files (preferred over git add -A)
git add <file1> <file2> <file3>

# Create commit with heredoc for proper formatting
git commit -m "$(cat <<'EOF'
<type>: <summary>

<optional detailed body>

EOF
)"

# Verify commit was created
git status
```

## Commit Message Format

```
<type>: <short summary in imperative mood>

<optional longer description explaining WHY, not WHAT>

```

### Types

- `feat`: New feature
- `fix`: Bug fix
- `test`: Add or update tests
- `docs`: Documentation changes
- `refactor`: Code refactoring
- `chore`: Maintenance tasks
- `perf`: Performance improvements

### Examples

```
test: add comprehensive HTTP mocking tests for API client

- Add 19 new wiremock-based tests for src/api/client.rs
- Improve coverage from 34.1% to 88.9% (+54.8pp)
- Test all search parameters, error handling, and pagination
- All tests properly mocked with NO real API calls

```

```
feat: add HTTP server with API key authentication

Add axum-based HTTP API server exposing all CLI functionality.
Includes background daemon support and request timeout config.

```

## Safety Rules

- **NEVER** use `--no-verify` or skip hooks
- **NEVER** commit sensitive files (.env, credentials, tokens)
- **NEVER** use `git add -A` or `git add .` (add specific files)
- **NEVER** amend commits unless explicitly requested
- **NEVER** force push to main/master

## When $ARGUMENTS is provided

Use it as the commit summary and follow the same format:

```bash
git commit -m "$(cat <<'EOF'
$ARGUMENTS
EOF
)"
```
