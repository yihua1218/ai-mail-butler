# Microsoft 365 (M365) OAuth 2.0 設定指南

由於微軟正在逐步淘汰基本驗證（使用單一密碼的 SMTP AUTH），讓 AI Mail Butler 代表您寄信最現代且安全的方式是透過 OAuth 2.0 並使用 **Microsoft Graph API**。

本指南將說明如何在 Azure Active Directory (Entra ID) 中註冊您的應用程式，並取得必要的授權憑證。

## 第一步：在 Microsoft Entra ID 註冊應用程式

1. 使用您的管理員帳號登入 [Microsoft Entra 系統管理中心](https://entra.microsoft.com/)。
2. 導覽至左側選單的 **應用程式** > **應用程式註冊** > **新增註冊**。
3. 填寫註冊表單：
   - **名稱**：`AI Mail Butler`（或您偏好的名稱）。
   - **支援的帳戶類型**：選擇「僅此組織目錄中的帳戶 (僅限單一租用戶)」。
   - **重新導向 URI (Redirect URI)**：選擇 **Web**，並輸入您的回呼網址。在本地開發階段請輸入 `http://localhost:3000/api/auth/m365/callback`。正式上線時請更新為您的實際網域。
4. 點擊 **註冊**。

## 第二步：取得核心憑證

應用程式註冊完成後，您會進入總覽頁面。
1. 複製 **應用程式 (用戶端) 識別碼 (Client ID)**。
2. 複製 **目錄 (租用戶) 識別碼 (Tenant ID)**。
3. 導覽至左側的 **憑證及秘密**。
4. 點擊 **新增用戶端密碼**，輸入描述並選擇過期時間。
5. 點擊 **新增**。**請立刻將密碼的「值」(Value) 複製下來**（一旦離開此頁面，您將無法再次查看此值）。

## 第三步：設定 API 權限

我們需要授與應用程式透過 Microsoft Graph API 代表您寄信的權限。

1. 導覽至左側的 **API 權限**。
2. 點擊 **新增權限**。
3. 選擇 **Microsoft Graph**。
4. 選擇 **委派的權限 (Delegated permissions)**（這代表應用程式將「代表登入的使用者」執行動作）。
5. 搜尋並勾選以下權限：
   - `Mail.Send`（允許發送電子郵件）
   - `User.Read`（允許讀取基本使用者設定檔資料）
6. 點擊 **新增權限**。
7. **關鍵步驟**：點擊畫面上方的 **代表 [您的組織名稱] 授與管理員同意** 按鈕，讓這些權限正式生效。

## 第四步：環境變數設定

將您收集到的憑證加入 AI Mail Butler 專案根目錄下的 `.env` 檔案中：

```env
M365_CLIENT_ID=your-client-id
M365_CLIENT_SECRET=your-client-secret
M365_TENANT_ID=your-tenant-id
```

設定完成後，後端將實作 OAuth 2.0 授權碼流程來取得 Access Token，並透過 Microsoft Graph 的 `POST /me/sendMail` 端點來寄送 AI 自動生成的回信。
