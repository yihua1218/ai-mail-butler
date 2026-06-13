# 每日自動檢查

這份檢查用來每天確認 dev 服務、AI model、財務 API，以及本機同步資料庫的關鍵財務 invariant 是否正常。

## 檢查內容

`scripts/daily-check.sh` 會檢查：

- `PUBLIC_URL` 首頁 HTTP 狀態。
- `/api/finance/records` HTTP 狀態與回傳結構。
- `/api/finance/monthly` HTTP 狀態與回傳結構。
- `ai-mail-butler` container 是否正在執行。
- container 內 `AI_MODEL_NAME` 是否符合預期。
- Uber / UberEats 財務明細筆數是否大於等於基準值。
- Uber / UberEats 財務明細總額是否大於等於基準值。
- `monthly_finance_summary` 與 `email_financial_records` 的月統計是否一致。

檢查結果會寫成 JSON report，預設位置：

```bash
logs/daily-checks/YYYYmmddTHHMMSSZ.json
```

## 手動執行

```bash
./scripts/daily-check.sh
```

只想測試、不想把 report 留在 repo 目錄時：

```bash
DAILY_CHECK_REPORT_PATH=/tmp/ai-mail-butler-daily-check.json ./scripts/daily-check.sh
```

成功時 exit code 是 `0`；失敗時 exit code 是 `1`，並且會把 failure list 印到 stderr。

## 可調整參數

可用環境變數覆蓋預設值：

```bash
DAILY_CHECK_PUBLIC_URL=https://butler-dev.yihua.app
DAILY_CHECK_EMAIL=yihua1218@gmail.com
DAILY_CHECK_CONTAINER_NAME=ai-mail-butler
DAILY_CHECK_EXPECTED_MODEL=google/gemma-4-31b-qat
DAILY_CHECK_DB_PATH=/home/nier/workspace/ai-mail-butler/ai-mail-butler-data/overlay/data/data.sqlite
DAILY_CHECK_MIN_UBER_COUNT=66
DAILY_CHECK_MIN_UBER_SUM=25328
DAILY_CHECK_MAX_SUMMARY_DELTA=0.01
DAILY_CHECK_REPORT_DIR=/home/nier/workspace/ai-mail-butler/logs/daily-checks
```

若同步資料新增了更多 Uber / UberEats 信件，建議提高 `DAILY_CHECK_MIN_UBER_COUNT` 與 `DAILY_CHECK_MIN_UBER_SUM`，讓每日檢查能抓到退化。

## 安裝 systemd user timer

從 repo root 執行：

```bash
mkdir -p ~/.config/systemd/user
cp systemd/user/ai-mail-butler-daily-check.service ~/.config/systemd/user/
cp systemd/user/ai-mail-butler-daily-check.timer ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now ai-mail-butler-daily-check.timer
```

查看排程：

```bash
systemctl --user list-timers ai-mail-butler-daily-check.timer
```

手動跑一次：

```bash
systemctl --user start ai-mail-butler-daily-check.service
```

查看 logs：

```bash
journalctl --user -u ai-mail-butler-daily-check.service -n 100 --no-pager
```

## 維運注意事項

- 這個檢查目前以 dev host 的本機 overlay DB 為基準。
- report 不會包含 API key 或 SMTP password。
- `nerdctl exec` 只讀取 `AI_MODEL_NAME`，不輸出其他環境變數。
- 如果 dev 服務改用 Docker 而不是 nerdctl，腳本需要增加 Docker fallback。
- 如果 `.env` 改回 container 內 SSHFS mount，需先確認 container 有安全可接受的 FUSE 設定。
