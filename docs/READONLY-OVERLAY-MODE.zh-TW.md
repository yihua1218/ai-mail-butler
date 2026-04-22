# 唯讀疊加模式（Readonly Overlay Mode）

唯讀疊加模式可讓您對現有的資料快照執行 AI Mail Butler，而不會對其造成任何修改。所有寫入操作都會被導向獨立的疊加目錄（overlay directory）；讀取時若疊加目錄中尚無對應檔案，系統會自動透明地 fallback 回原始（base）資料。SQLite 資料庫在首次啟動時會從 base 快照複製一份到 overlay 目錄。

適用場景：
- **Demo 環境** — 展示真實感的資料，同時完全杜絕意外變更。
- **Staging / QA** — 用生產環境快照測試新版應用，而不影響正式資料。
- **唯讀鏡像** — 對受限使用者提供資料存取，同時禁止所有寫入操作。

---

## 運作原理

### 三層保護機制

| 層級 | 說明 |
|---|---|
| **API 寫入防護** | 中介層 (Middleware) 會攔截所有 `POST`、`PUT`、`DELETE` 請求（驗證相關端點除外），並在唯讀模式下回傳 `503 Service Unavailable`。 |
| **檔案路徑重映射** | 所有檔案寫入（SMTP spool、郵件封存、附件、解碼段落）都會被重導向至 overlay 目錄，而非原始邏輯路徑。 |
| **Union 讀取語義** | 讀取檔案時優先查找 overlay；若目標檔案不存在，系統會自動 fallback 到 `readonly_base` 下的對應路徑。Spool 目錄列表會合併兩個目錄的內容（overlay 版本優先，依檔名去重）。 |

### 資料庫（SQLite）

首次以唯讀模式啟動時，應用程式會將 base 資料庫檔案複製至 `<overlay_dir>/data/data.sqlite`。後續所有讀寫操作僅使用此 overlay 副本，base 快照始終保持不變。

---

## 設定方式

### 環境變數

| 變數名稱 | 預設值 | 說明 |
|---|---|---|
| `READONLY_MODE` | `false` | 設定為 `true`、`1`、`yes` 或 `on` 即可啟用唯讀疊加模式。 |
| `READONLY_BASE` | （空值） | base 資料快照目錄的絕對或相對路徑。讀取時從此根目錄 fallback。 |
| `OVERLAY_DIR` | `data/overlay` | 所有寫入與 overlay DB 副本的存放目錄。不存在時自動建立。 |

### CLI 參數

```bash
cargo run -- \
  --readonly-mode \
  --readonly-base /path/to/production-snapshot \
  --overlay-dir /tmp/readonly-overlay
```

| 參數 | 說明 |
|---|---|
| `--readonly-mode` | 啟用唯讀疊加模式（等同於 `READONLY_MODE=true`）。 |
| `--readonly-base <path>` | base 快照目錄路徑。 |
| `--overlay-dir <path>` | overlay 輸出目錄路徑。 |

同時設定環境變數與 CLI 參數時，CLI 參數優先。

---

## 範例：Demo 模式

```bash
# 將生產快照複製到唯讀位置
cp -r /var/lib/ai-mail-butler /srv/demo-snapshot

# 以唯讀模式啟動應用程式
READONLY_MODE=true \
READONLY_BASE=/srv/demo-snapshot \
OVERLAY_DIR=/tmp/demo-overlay \
cargo run
```

啟動時，應用程式會依序執行：
1. 將 `/srv/demo-snapshot/data/data.sqlite` 複製至 `/tmp/demo-overlay/data/data.sqlite`。
2. 將所有檔案寫入重導向至 `/tmp/demo-overlay/` 下的對應路徑。
3. 攔截所有 API 寫入請求（回傳 `503`）。
4. 讀取時先查 `/tmp/demo-overlay/`，找不到再自動 fallback 至 `/srv/demo-snapshot/`。

---

## 範例：Docker Compose

```yaml
services:
  app:
    image: ai-mail-butler:latest
    environment:
      READONLY_MODE: "true"
      READONLY_BASE: "/data/base"
      OVERLAY_DIR: "/data/overlay"
    volumes:
      - ./production-snapshot:/data/base:ro
      - overlay-vol:/data/overlay
volumes:
  overlay-vol:
```

---

## 前端顯示

啟用唯讀模式時：

- 每個頁面頂端都會顯示**黃色警告橫幅**，說明目前為唯讀模式，並顯示 overlay 及 base 路徑。
- **About（關於）** 頁面會顯示目前的執行模式（`Readonly Overlay` 或 `Normal`）。

---

## API

`/api/about` 端點會包含唯讀模式的相關資訊：

```json
{
  "readonly_mode_enabled": true,
  "readonly_base": "/srv/demo-snapshot",
  "overlay_dir": "/tmp/demo-overlay"
}
```

---

## Union 讀取語義（詳細說明）

### Spool 檔案列表

`union_list_eml_files` 會合併 overlay spool 與 base spool 中的 `.eml` 檔案：

1. 優先收集 `<overlay_dir>/data/mail_spool/` 中的所有 `.eml` 檔案。
2. 從 `<readonly_base>/data/mail_spool/` 中，將**檔名尚未出現在 overlay 集合中**的 `.eml` 檔案補充加入。
3. 最終列表依路徑排序。

規則：
- 同時存在於兩個位置的檔案，以 **overlay 版本**（即本地寫入或修改的版本）為準。
- 僅存在於 base 的檔案，以唯讀方式提供。

### 檔案讀取 Fallback

`union_read_file` 會先嘗試從 overlay 路徑讀取。若檔案不存在且目前為唯讀模式，系統會自動構建 `readonly_base` 下的對應路徑並重試。此機制用於背景 Spool 處理器讀取 `.eml` 檔案時。

### 目錄統計（資料刪除快照）

`collect_dir_stats` 同時掃描 overlay 寄件人目錄與 base 對應目錄，因此資料刪除快照中回報的檔案數量與總大小，皆反映兩個目錄的聯集結果。

---

## 限制說明

- 唯讀模式在**應用程式層**保護檔案 I/O 及 API 寫入。它不會強制執行 base 目錄的 OS 層檔案系統權限（建議在 Docker 中將 base 掛載為唯讀 `:ro`）。
- Overlay DB 副本是 base DB 的完整複製。啟動時執行的 Schema Migration 只會修改 overlay DB 副本，不影響 base。
- 唯讀模式**不會**封鎖 SMTP 接收（連接埠 25）— 傳入的郵件仍會被寫入 overlay spool 並正常處理。如需封鎖對外 SMTP 流量，請另行設定防火牆規則。
