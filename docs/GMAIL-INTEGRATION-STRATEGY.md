## AI Mail Butler: Gmail Integration Strategy and Compliance Guide

### Overview
Implementation of automated email replies for Gmail users involves navigating various authorization methods and Google's security audit requirements. This document outlines the technical roadmap from development to production.

---

### Phase 1: Current Implementation (SMTP with App Passwords)
The existing architecture utilizes the **SMTP protocol** paired with **Google App Passwords**.

* **Mechanism**: Users generate a 16-digit App Password in Google Account settings and provide it to the assistant.
* **Pros**:
    * Zero cost; no security audit required.
    * Full control over the Rust-based SMTP client.
* **Cons**:
    * Requires Manual user configuration (2FA must be enabled).
    * The `From` header is restricted to the account owner's email address or pre-verified aliases.
* **Status**: **Production-ready for self-hosted or private use.**

---

### Phase 2: Intermediate Scaling (OAuth 2.0 Testing Mode)
To improve user experience, the assistant can transition to **OAuth 2.0** without immediate formal auditing.

* **Mechanism**: Applications are kept in **"Testing"** status within the Google Cloud Console.
* **Key Constraints**:
    * **User Limit**: Restricted to a maximum of **100 specific test users** (must be manually whitelisted by the developer).
    * **Token Expiry**: Refresh tokens may expire every **7 days** for unverified apps, requiring periodic re-authentication.
* **Integration Scope**:
    * `https://www.googleapis.com/auth/gmail.send` (Send emails on behalf of the user).
    * `https://www.googleapis.com/auth/gmail.readonly` (Read incoming emails for processing).
* **Status**: **Recommended for Beta testing and limited pilot groups.**

---

### Phase 3: Production and Public Release (CASA Compliance)
For a wide-scale public release, the application must undergo the **Cloud App Security Assessment (CASA)**.

#### Restricted Scopes and Audits
Any application accessing, aggregating, or transferring Gmail data is subject to the **Restricted Scope** policy.

| Audit Tier | Target Audience           | Estimated Annual Cost | Requirement                                                   |
| :--------- | :------------------------ | :-------------------- | :------------------------------------------------------------ |
| **Tier 2** | Public SaaS / High Volume | $15,000 - $75,000 USD | Third-party penetration testing and vulnerability assessment. |

#### Strategies to Minimize Friction
1.  **Scope Minimization**: Request only `gmail.send` if the assistant only needs to reply. This may fall under "Sensitive" rather than "Restricted" scopes in certain contexts, potentially lowering audit complexity.
2.  **User-Provisioned Credentials**: Documentation can be provided to guide advanced users in creating their own **Google Cloud Client IDs**. This shifts the compliance burden to the end-user while keeping the software open-source and free of centralized audit costs.

---

### Technical Implementation Comparison



| Feature            | App Password (SMTP)           | OAuth 2.0 (Gmail API)            |
| :----------------- | :---------------------------- | :------------------------------- |
| **Security**       | Medium (Permanent Credential) | High (Short-lived Tokens)        |
| **Setup Ease**     | Manual / Complex for Users    | One-click Authorization          |
| **Audit Required** | No                            | Yes (CASA for Restricted Scopes) |
| **API Features**   | Send/Receive only             | Full Threading, Labels, Drafts   |

---

### Recommended Action Plan
1.  **Maintain** the current SMTP/App Password logic as the default fallback for self-hosted users.
2.  **Implement** an OAuth 2.0 flow in the Rust backend using `openidconnect` or `oauth2` crates.
3.  **Provide** a configuration toggle to allow users to input their own `CLIENT_ID` and `CLIENT_SECRET` to leverage OAuth benefits without a centralized audited app.