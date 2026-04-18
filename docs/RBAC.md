# Role-Based Access Control (RBAC)

AI Mail Butler implements a robust Role-Based Access Control system to ensure data privacy and secure management. The system is divided into three distinct roles:

## 1. Admin (管理員)

The system administrator has full access to the AI Mail Butler platform. 

- **Identification**: The admin is identified by matching their login email against the `ADMIN_EMAIL` environment variable set in the `.env` file.
- **Permissions**: 
  - **Dashboard**: Can view the overall system statistics (total users, total received emails, total replied emails).
  - **Emails**: Can view a comprehensive list of all emails processed by the system, regardless of the recipient user.

*To set up an admin, add the following to your `.env` file:*
```env
ADMIN_EMAIL=admin@example.com
```

## 2. Registered User (一般使用者)

Regular users who use the AI Mail Butler to process their forwarded emails.

- **Identification**: Any user who has successfully logged in via the Magic Link and is registered in the database, but whose email does not match `ADMIN_EMAIL`.
- **Permissions**:
  - **Dashboard**: Can only view their own personal dashboard.
  - **Emails**: Can exclusively view emails that were forwarded by or addressed to them. They have absolutely no access to system statistics or other users' emails.

## 3. Anonymous (匿名使用者)

Visitors who have not authenticated.

- **Identification**: Users accessing the Web UI without a valid Magic Link token.
- **Permissions**:
  - **Dashboard**: Sees only a public welcome landing page prompting them to log in.
  - **Emails & Stats**: Completely restricted. No emails or system statistics are exposed to unauthenticated visitors.
