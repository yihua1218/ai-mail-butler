---
mode: agent
description: Analyze a requirements document and produce an implementation plan and project skeleton.
---

# Requirements Review and Processing Workflow

This Workflow provides a standardized process for analyzing and processing system requirement documents (such as `requirements.md`), translating them into actionable implementation plans and system architectural skeletons.

## 1. Content Analysis and Comprehension

- **Read Document**: Thoroughly read all sections in `requirements.md`, including project goals, core functionalities, non-functional requirements, and acceptance criteria.
- **Extract Key Elements**:
  - Core modules (e.g., Mail Server, Web GUI, AI Engine)
  - User flows (e.g., Forwarding -> Identification -> Onboarding -> AI Processing -> Web Display)
  - Performance and security requirements (e.g., Passwordless login, local data processing, etc.)

## 2. Technology Selection and Architecture Mapping

- **Technology Mapping**: Map the requirements to appropriate technology stacks.
  - Example: High performance and asynchronous needs -> **Rust**
  - Web API and Dashboard -> **Axum** or **Actix-Web**
  - Database Storage -> **SQLite** / **PostgreSQL** + **SQLx**
- **Module Breakdown**: Plan the system directory structure based on core functionalities (e.g., split into `mail`, `ai`, `db`, `web`, `services`).

## 3. Create Implementation Plan

- Draft the implementation plan to clearly define the system architecture.
- Propose open questions to confirm architectural details with the user (e.g., method of receiving emails: built-in SMTP or Webhook, frontend rendering approach, etc.).

## 4. Project Skeleton Initialization

After confirming the plan, execute the following to set up the project skeleton:
1. **Create Config Files**: `Cargo.toml`, `.gitignore`, `.env.example`
2. **Initialize Source Directory**: Create the `src/` directory and place `mod.rs` for each submodule along with basic skeletons.
3. **Write README.md**: Consolidate requirements and architecture, updating the project's README file.
4. **Prepare Task List**: Put the remaining development work into a Task list for tracking.
