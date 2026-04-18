# SMTP Setup for Gmail / Gmail SMTP 設定指南

This guide explains how to configure Gmail as an SMTP relay for AI Mail Butler using **App Passwords**.

本指南說明如何使用 **應用程式密碼** 設定 Gmail 作為 AI Mail Butler 的 SMTP 轉發伺服器。

---

## English Instructions

To use Gmail as an SMTP relay, you need to create an **App Password**. This is required because Google no longer supports "Less secure apps" and requires 2-Step Verification for SMTP authentication.

### Prerequisites
1.  **2-Step Verification** must be enabled for your Google account.

### Step 1: Create an App Password
1.  Go to your [Google Account](https://myaccount.google.com/) settings.
2.  Select **Security** on the left menu.
3.  Under the "How you sign in to Google" section, select **2-Step Verification**.
4.  Scroll to the very bottom of the page and select **App passwords**.
5.  Enter a custom name for the app (e.g., "AI Mail Butler").
6.  Click **Create**.
7.  Copy the generated 16-character password in the yellow box. **Remove any spaces when pasting into your configuration.**

### Step 2: Configure `.env`
Update your `.env` file in the project root:
```env
SMTP_RELAY_HOST=smtp.gmail.com
SMTP_RELAY_PORT=587
SMTP_RELAY_USER=your-email@gmail.com
SMTP_RELAY_PASS=your-16-character-app-password
```

---

## 繁體中文說明

如果您想使用 Gmail 作為 SMTP 轉發伺服器，您需要建立 **應用程式密碼 (App Password)**。由於 Google 已停止支援「安全性較低的應用程式」，且要求必須啟用兩步驟驗證才能使用 SMTP 驗證。

### 前置作業
1.  您的 Google 帳號必須已啟用 **兩步驟驗證 (2-Step Verification)**。

### 第一步：建立應用程式密碼
1.  前往您的 [Google 帳戶](https://myaccount.google.com/) 設定。
2.  在左側選單中選擇 **安全性 (Security)**。
3.  在「登入 Google 的方式」區塊中，點擊 **兩步驟驗證 (2-Step Verification)**。
4.  捲動到頁面最下方，點擊 **應用程式密碼 (App passwords)**。
5.  輸入應用程式自訂名稱（例如 "AI Mail Butler"）。
6.  點擊 **建立 (Create)**。
7.  複製黃色方塊中顯示的 16 位元密碼。**填入設定時請移除中間的空格。**

### 第二步：設定 `.env`
將生成的資訊填入專案根目錄的 `.env`：
```env
SMTP_RELAY_HOST=smtp.gmail.com
SMTP_RELAY_PORT=587
SMTP_RELAY_USER=您的 Gmail 地址
SMTP_RELAY_PASS=剛才產生的 16 位元密碼
```
