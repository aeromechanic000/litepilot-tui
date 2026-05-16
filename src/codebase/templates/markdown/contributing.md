<!-- @LITE_DESC: Contributing guide template for open source projects -->
<!-- @LITE_SCENE: Open source projects, team collaboration, community guidelines -->
<!-- @LITE_TAGS: markdown, contributing, open-source, community, guidelines -->

# Contributing to Project Name

Thank you for your interest in contributing! This guide covers how to set up the project and submit contributions.

## Quick Start

1. **Fork** the repository
2. **Clone** your fork: `git clone https://github.com/your-username/project.git`
3. **Install** dependencies: `npm install`
4. **Create** a branch: `git checkout -b feature/your-feature`
5. **Develop** your changes
6. **Test**: `npm test`
7. **Submit** a Pull Request

## Development Setup

### Prerequisites

- Node.js >= 18 (or Python >= 3.10, Rust >= 1.70, etc.)
- Git
- Docker (for integration tests)

### Local Development

```bash
# Install dependencies
npm install

# Run in development mode
npm run dev

# Run tests
npm test

# Run linter
npm run lint

# Build for production
npm run build
```

## Code Style

- Follow the existing code style in the project
- Use meaningful variable and function names
- Add comments only for non-obvious logic
- Keep functions focused and concise

## Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add user authentication
fix: resolve memory leak in data processing
docs: update API documentation
test: add integration tests for user endpoints
refactor: simplify database query builder
```

## Pull Request Process

1. Update documentation for any new features
2. Add tests for new functionality
3. Ensure all existing tests pass
4. Keep PRs focused on a single change
5. Link related issues in the PR description

## Reporting Bugs

When filing a bug report, please include:

- **OS and version** (e.g., macOS 14.2, Ubuntu 22.04)
- **Steps to reproduce** the issue
- **Expected behavior** vs. **actual behavior**
- **Logs or error messages** (sanitized of sensitive data)

## License

By contributing, you agree that your contributions will be licensed under the same license as the project.
