# Cloudflare Cache Purge Token 與快取清除操作

本文件說明如何建立只能清除 Cloudflare cache 的 API Token、如何設定 AI Mail Butler，以及部署後如何只清除指定項目的快取。

## 建立只能 Purge Cache 的 API Token

1. 登入 Cloudflare。
2. 進入 **My Profile** -> **API Tokens**。
3. 點選 **Create Token**。
4. 選擇 **Create Custom Token**。
5. Token 名稱建議填 `AI Mail Butler Cache Purge`。
6. 權限只設定：
   - **Zone** -> **Cache Purge** -> **Purge**
7. Zone resources 設定：
   - **Include** -> **Specific zone** -> 你的 zone，例如 `yihua.app`
8. 不要加入 DNS、Workers、Page Rules、Account 或任何 edit 權限。
9. 建立 token，並只複製保存一次。

這個 Token 不應該能修改 DNS、調整 Zone 設定、部署 Workers，或讀取其他帳號資源。它唯一的用途就是對指定 zone 執行 cache purge。

## 執行環境變數

在部署環境的 `.env` 加入：

```env
PUBLIC_URL=https://butler.yihua.app
CLOUDFLARE_ZONE_ID=your-cloudflare-zone-id
CLOUDFLARE_API_TOKEN=your-cache-purge-only-token
```

`PUBLIC_URL` 會用來組出部分清除快取時的完整 URL。

## 從 Admin Dashboard 清除

Admin 與 Developer 使用者登入 Dashboard 後，可以使用 **Cloudflare 快取清除**。

可選項目：

- **只清 WebLLM 本地端頁面**：清除 `https://butler.yihua.app/webllm-local`
- **只清 Browser Extension ZIP**：清除 `https://butler.yihua.app/downloads/ai-mail-butler-browser-extension-0.1.0.zip`
- **前端頁面**：清除主要 SPA 頁面 URL：`/`、`/dashboard`、`/chat`、`/settings`、`/rules`、`/finance`、`/privacy`、`/how-it-works`、`/webllm-local`、`/about`
- **整個網站**：送出 Cloudflare `purge_everything`

## 手動部分清除範例

執行前請替換 `ZONE_ID` 與 `TOKEN`。

只清 WebLLM 本地端頁面：

```bash
curl -X POST "https://api.cloudflare.com/client/v4/zones/ZONE_ID/purge_cache" \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  --data '{"files":["https://butler.yihua.app/webllm-local"]}'
```

只清 Browser Extension ZIP：

```bash
curl -X POST "https://api.cloudflare.com/client/v4/zones/ZONE_ID/purge_cache" \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  --data '{"files":["https://butler.yihua.app/downloads/ai-mail-butler-browser-extension-0.1.0.zip"]}'
```

清前端頁面：

```bash
curl -X POST "https://api.cloudflare.com/client/v4/zones/ZONE_ID/purge_cache" \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  --data '{"files":["https://butler.yihua.app/","https://butler.yihua.app/dashboard","https://butler.yihua.app/chat","https://butler.yihua.app/settings","https://butler.yihua.app/rules","https://butler.yihua.app/finance","https://butler.yihua.app/privacy","https://butler.yihua.app/how-it-works","https://butler.yihua.app/webllm-local","https://butler.yihua.app/about"]}'
```

清整個網站：

```bash
curl -X POST "https://api.cloudflare.com/client/v4/zones/ZONE_ID/purge_cache" \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  --data '{"purge_everything":true}'
```

## 部署注意事項

GitHub workflow 會在 frontend build 前先打包 Browser Extension ZIP。部署後 image 應該包含：

```text
frontend/dist/downloads/ai-mail-butler-browser-extension-0.1.0.zip
```

如果剛部署新的 Browser Extension 封裝，請清除 **只清 Browser Extension ZIP**，確保使用者下載到最新 zip。
