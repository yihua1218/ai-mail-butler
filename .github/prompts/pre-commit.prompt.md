---
mode: agent
description: Safely prepare a public commit — compile, test, sanitize sensitive data, and verify docs.
---

# Pre-Commit Preparation Workflow

This workflow ensures that the codebase is compiling correctly and is completely free of any sensitive data before performing a public commit.

1. Read the `README.md` to identify the documented commands for Compiling and Testing (e.g., `cargo build` and `cargo test`).
2. Execute those exact commands to ensure the project correctly compiles and all tests pass without errors.
3. Identify all files in the current workspace that are currently tracked or will be tracked by git (i.e., not excluded by `.gitignore`).
4. Read through the contents of these files to identify any sensitive information. This includes:
   - Private IP addresses or specific domains not meant for public viewing.
   - Hardcoded API keys, secrets, plain-text passwords, or authentication tokens.
   - Absolute local file paths that reveal the developer's personal environment or usernames.
5. If any sensitive data is found, automatically sanitize the data (e.g., replace IPs with `localhost`, replace tokens with `your-token-here`).
6. Verify that configuration files (like `docker-compose.yml`, `Dockerfile`) are using English comments (translate any non-English comments to English) and use placeholder credentials.
7. Ensure all workflow files under the `.agents/workflows/` directory are written in English (translate any non-English workflow files to English).
8. Check if a `LICENSE` file exists. If it does not exist, generate one based on the license described in the `README.md`.
9. Check that primary documentation files (`README.md`, `TODO.md`) have synchronized Traditional Chinese translations in `*.zh-TW.md` format.
10. Provide a concise summary of the checked files, any sanitizations performed, and confirm if the project is ready for `git commit`.
