# Cloudflare Cache Purge Token and Operations

This guide explains how to create a Cloudflare API Token that can only purge cache for one zone, how to configure AI Mail Butler, and how to purge focused cache targets after deployment.

## Create a Cache-Purge-Only API Token

1. Log in to Cloudflare.
2. Open **My Profile** -> **API Tokens**.
3. Click **Create Token**.
4. Choose **Create Custom Token**.
5. Name it `AI Mail Butler Cache Purge`.
6. Set permissions:
   - **Zone** -> **Cache Purge** -> **Purge**
7. Set zone resources:
   - **Include** -> **Specific zone** -> your zone, for example `yihua.app`
8. Do not add DNS, Workers, Page Rules, Account, or edit permissions.
9. Create the token and copy it once.

This token should not be able to modify DNS, change zone settings, deploy Workers, or read unrelated account resources. Its only purpose is cache purge for the selected zone.

## Runtime Environment Variables

Add these to the deployed `.env`:

```env
PUBLIC_URL=https://butler.yihua.app
CLOUDFLARE_ZONE_ID=your-cloudflare-zone-id
CLOUDFLARE_API_TOKEN=your-cache-purge-only-token
```

`PUBLIC_URL` is used to build exact URLs for partial purge requests.

## Purge from Admin Dashboard

Admin and Developer users can open Dashboard and use **Cloudflare Cache Purge**.

Available targets:

- **WebLLM local page only**: purges `https://butler.yihua.app/webllm-local`
- **Browser extension ZIP only**: purges `https://butler.yihua.app/downloads/ai-mail-butler-browser-extension-0.1.0.zip`
- **Frontend pages**: purges the main SPA page URLs: `/`, `/dashboard`, `/chat`, `/settings`, `/rules`, `/finance`, `/privacy`, `/how-it-works`, `/webllm-local`, and `/about`
- **Entire site**: sends Cloudflare `purge_everything`

## Manual Partial Purge Examples

Replace `ZONE_ID` and `TOKEN` before running.

Only purge the WebLLM local page:

```bash
curl -X POST "https://api.cloudflare.com/client/v4/zones/ZONE_ID/purge_cache" \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  --data '{"files":["https://butler.yihua.app/webllm-local"]}'
```

Only purge the browser extension ZIP:

```bash
curl -X POST "https://api.cloudflare.com/client/v4/zones/ZONE_ID/purge_cache" \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  --data '{"files":["https://butler.yihua.app/downloads/ai-mail-butler-browser-extension-0.1.0.zip"]}'
```

Purge frontend pages:

```bash
curl -X POST "https://api.cloudflare.com/client/v4/zones/ZONE_ID/purge_cache" \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  --data '{"files":["https://butler.yihua.app/","https://butler.yihua.app/dashboard","https://butler.yihua.app/chat","https://butler.yihua.app/settings","https://butler.yihua.app/rules","https://butler.yihua.app/finance","https://butler.yihua.app/privacy","https://butler.yihua.app/how-it-works","https://butler.yihua.app/webllm-local","https://butler.yihua.app/about"]}'
```

Purge the entire site:

```bash
curl -X POST "https://api.cloudflare.com/client/v4/zones/ZONE_ID/purge_cache" \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  --data '{"purge_everything":true}'
```

## Deployment Note

The GitHub workflow packages the browser extension ZIP before frontend build. The deployed image should contain:

```text
frontend/dist/downloads/ai-mail-butler-browser-extension-0.1.0.zip
```

After deploying a new browser extension package, purge the **Browser extension ZIP only** target so users download the latest archive.
