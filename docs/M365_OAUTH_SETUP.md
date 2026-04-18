# Microsoft 365 (M365) OAuth 2.0 Setup Guide

Since Microsoft is deprecating Basic Authentication (SMTP AUTH with simple passwords), the modern and secure way to allow AI Mail Butler to send emails on your behalf is through OAuth 2.0 using the **Microsoft Graph API**.

This guide explains how to register your application in Azure Active Directory (Entra ID) and obtain the necessary credentials.

## Step 1: Register the Application in Microsoft Entra ID

1. Log in to the [Microsoft Entra admin center](https://entra.microsoft.com/) using your administrator account.
2. Navigate to **Applications** > **App registrations** > **New registration**.
3. Fill out the form:
   - **Name**: `AI Mail Butler` (or any preferred name).
   - **Supported account types**: Select "Accounts in this organizational directory only (Single tenant)".
   - **Redirect URI**: Select **Web** from the dropdown and enter your callback URL. For local development, use `http://localhost:3000/api/auth/m365/callback`. Update this to your production domain later.
4. Click **Register**.

## Step 2: Retrieve Core Credentials

After the application is registered, you will be taken to its overview page.
1. Copy the **Application (client) ID**.
2. Copy the **Directory (tenant) ID**.
3. Navigate to **Certificates & secrets** in the left sidebar.
4. Click **New client secret**, provide a description, and choose an expiration duration.
5. Click **Add**. **Immediately copy the `Value` of the secret** (you will not be able to see it again once you leave the page).

## Step 3: Configure API Permissions

We need to grant the application permission to send emails on your behalf via the Microsoft Graph API.

1. Navigate to **API permissions** in the left sidebar.
2. Click **Add a permission**.
3. Select **Microsoft Graph**.
4. Select **Delegated permissions** (this means the app acts on behalf of the logged-in user).
5. Search for and check the following permissions:
   - `Mail.Send` (allows sending emails)
   - `User.Read` (allows reading basic user profile data)
6. Click **Add permissions**.
7. **Crucial Step**: Click the **Grant admin consent for [Your Organization]** button to actively apply these permissions.

## Step 4: Environment Variables

Add the credentials you collected to your `.env` file in the root of the AI Mail Butler project:

```env
M365_CLIENT_ID=your-client-id
M365_CLIENT_SECRET=your-client-secret
M365_TENANT_ID=your-tenant-id
```

Once configured, the backend will implement the OAuth 2.0 Authorization Code Flow to acquire access tokens and send AI-generated replies via the Microsoft Graph `POST /me/sendMail` endpoint.
