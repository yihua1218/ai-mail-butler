# SMTP Setup for Microsoft 365 (M365) / Microsoft 365 SMTP 設定指南

This guide explains how to configure Microsoft 365 as an SMTP relay for AI Mail Butler using **App Passwords**.

本指南說明如何使用 **應用程式密碼** 設定 Microsoft 365 作為 AI Mail Butler 的 SMTP 轉發伺服器。

---

## English Instructions

To use Microsoft 365 as an SMTP relay, you need to set up an **App Password**. This is required when Multi-Factor Authentication (MFA) is enabled.

### Prerequisites
1.  **Multi-Factor Authentication (MFA)** must be enabled for your account.
2.  **SMTP AUTH** must be enabled for your mailbox in the M365 Admin Center.

### Step 1: Enable SMTP AUTH (For Admins)
1.  Log in to the [Microsoft 365 Admin Center](https://admin.microsoft.com/).
2.  Go to **Users** > **Active users**.
3.  Select the user you want to use.
4.  In the flyout pane, go to **Mail** tab > **Manage email apps**.
5.  Ensure **Authenticated SMTP** is checked and click **Save changes**.

### Step 2: Create an App Password
1.  Go to your [My Account Security Info](https://mysignins.microsoft.com/security-info) page.
2.  Click **Add sign-in method**.
3.  Choose **App password** from the dropdown. 
    *   *Note: If you don't see this option, your organization may have "Security Defaults" enabled or has disabled App Passwords. Contact your IT administrator.*
4.  Give it a name (e.g., "AI Mail Butler") and click **Next**.
5.  Copy the generated password. **You will not be able to see it again.**

### Step 3: Configure `.env`
Update your `.env` file with the following:
```env
SMTP_RELAY_HOST=smtp.office365.com
SMTP_RELAY_PORT=587
SMTP_RELAY_USER=your-email@your-domain.com
SMTP_RELAY_PASS=your-generated-app-password
```

---

## Administrator: Enabling App Passwords for the Tenant

If users do not see "App password" in their security info, you (the admin) must enable it.

### 1. Disable "Security Defaults" (Required)
Microsoft "Security Defaults" block App Passwords. You must disable them to use this feature.
1.  Go to the [Microsoft Entra admin center](https://entra.microsoft.com/).
2.  Navigate to **Identity** > **Overview** > **Properties**.
3.  Click **Manage security defaults** at the bottom.
4.  Set **Security defaults** to **Disabled**. Click **Save**.
    *   *Warning: Ensure you have other MFA protections (like Conditional Access) if you disable this.*

### 2. Allow App Passwords in MFA Settings
1.  Go to the [Microsoft 365 Admin Center](https://admin.microsoft.com/).
2.  Navigate to **Users** > **Active users**.
3.  Click **Multi-factor authentication** in the top navigation bar.
4.  In the new window, click **service settings** at the top.
5.  Under **app passwords**, check **Allow users to create app passwords to sign in to non-browser apps**.
6.  Click **Save**.

---

## 繁體中文說明

如果您啟用了多重要素驗證 (MFA)，您需要建立 **應用程式密碼 (App Password)** 才能讓 AI Mail Butler 透過 Microsoft 365 寄信。

### 前置作業
1.  帳號必須已啟用 **多重要素驗證 (MFA)**。
2.  必須在 M365 管理中心為該信箱啟用 **SMTP AUTH**。

### 第一步：啟用 SMTP AUTH (管理員操作)
1.  登入 [Microsoft 365 管理中心](https://admin.microsoft.com/)。
2.  前往 **使用者 (Users)** > **現有使用者 (Active users)**。
3.  選擇您要使用的使用者。
4.  在右側視窗中，切換到 **郵件 (Mail)** 頁籤 > 點擊 **管理郵件應用程式 (Manage email apps)**。
5.  勾選 **已驗證的 SMTP (Authenticated SMTP)** 並點擊儲存變更。

### 第二步：建立應用程式密碼
1.  前往您的 [安全性資訊 (Security Info)](https://mysignins.microsoft.com/security-info) 頁面。
2.  點擊 **新增登入方法 (Add sign-in method)**。
3.  從下拉選單選擇 **應用程式密碼 (App password)**。
    *   *註：若沒看到此選項，請參考下方的「管理員：為組織啟用應用程式密碼功能」說明。*
4.  輸入名稱（例如 "AI Mail Butler"）並點擊下一步。
5.  立即複製生成的密碼。**關閉視窗後將無法再次查看。**

### 第三步：設定 `.env`
將生成的資訊填入專案根目錄的 `.env`：
```env
SMTP_RELAY_HOST=smtp.office365.com
SMTP_RELAY_PORT=587
SMTP_RELAY_USER=您的信箱地址
SMTP_RELAY_PASS=剛才產生的應用程式密碼
```

---

## 管理員：為組織啟用應用程式密碼功能

如果使用者在選單中找不到「應用程式密碼」，管理員必須手動開啟此權限。

### 1. 關閉「安全性預設值」(Security Defaults)
微軟的「安全性預設值」會強制執行 MFA 但同時會禁用應用程式密碼。
1.  登入 [Microsoft Entra 管理中心](https://entra.microsoft.com/)。
2.  前往 **身分 (Identity)** > **概觀 (Overview)** > **屬性 (Properties)**。
3.  點擊最下方的 **管理安全性預設值 (Manage security defaults)**。
4.  將狀態改為 **停用 (Disabled)** 並儲存。
    *   *注意：停用後建議改用「條件式存取」來維持組織安全。*

### 2. 在 MFA 設定中允許應用程式密碼
1.  登入 [Microsoft 365 管理中心](https://admin.microsoft.com/)。
2.  前往 **使用者 (Users)** > **現有使用者 (Active users)**。
3.  點擊上方工作列的 **多重要素驗證 (Multi-factor authentication)**。
4.  在開啟的新視窗中，點擊上方的 **服務設定 (service settings)**。
5.  在 **應用程式密碼 (app passwords)** 區塊，勾選 **允許使用者建立應用程式密碼以登入非瀏覽器的應用程式**。
6.  點擊 **儲存 (Save)**。
