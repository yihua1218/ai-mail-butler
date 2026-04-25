const LANG_KEY = 'extension_ui_lang';
const FOLLOW_WEBAPP_LANG_KEY = 'extension_follow_webapp_lang';
const WEBAPP_LANG_WHITELIST_KEY = 'extension_webapp_lang_whitelist';
const DEFAULT_WEBAPP_LANG_WHITELIST = [
  'https://butler.yihua.app',
  'http://localhost:5173',
  'http://127.0.0.1:5173',
];

const I18N = {
  en: {
    titleMain: 'AI Mail Butler Local',
    titleSub: 'All rules and logs stay in this browser profile.',
    langLabel: 'Language',
    followWebLangLabel: 'Follow Web App Language',
    syncLangNow: 'Sync Now',
    syncIdle: 'Language sync reads i18n_lang from the current active web app tab.',
    syncSuccess: 'Language synced from active tab',
    syncSuccessOrigin: 'Source origin',
    syncNoValue: 'No i18n_lang found in active tab localStorage.',
    syncFailed: 'Language sync failed',
    syncManualHint: 'Open your AI Mail Butler web app tab first, then click Sync Now.',
    syncPermissionDenied: 'This extension cannot access the current page.',
    syncPermissionNext: 'Open your AI Mail Butler web app tab, click the extension icon from that tab, and then try Sync Now again. If you are on another website, use manual language selection instead.',
    syncTabUnsupported: 'The current tab does not allow language sync.',
    syncWhitelistRejected: 'The current tab origin is not in the Web App whitelist.',
    syncWhitelistNext: 'Add this origin to the whitelist below, switch to that web app tab, and sync again.',
    webWhitelistTitle: 'Web App Origin Whitelist',
    webWhitelistHelp: 'Only these origins can be used for language sync.',
    webWhitelistPlaceholder: 'https://app.example.com',
    addWhitelistOrigin: 'Add',
    noWhitelistOrigins: 'No whitelist origins yet.',
    removeWhitelistOrigin: 'Remove',
    whitelistSaved: 'Whitelist updated.',
    whitelistInvalid: 'Please enter a valid origin, for example https://app.example.com',
    whitelistDuplicate: 'This origin is already in the whitelist.',
    whitelistCurrentOrigin: 'Current active origin',
    capabilityTitle: 'Permission & Capability Check',
    refresh: 'Refresh',
    capabilityChecking: 'Checking runtime capabilities...',
    capabilityReadyTitle: 'Ready',
    capabilityReadyDesc: 'Required APIs and permissions are available.',
    capabilityActionTitle: 'Action needed',
    capabilityActionDesc: 'Some permissions or APIs are unavailable. Certain automation features will be limited.',
    checkedAt: 'Checked at',
    repairGmailTab: 'Repair Current Gmail Tab',
    repairIdle: 'If Gmail actions fail after reloading the extension, repair the current Gmail tab.',
    repairNotGmail: 'The current active tab is not a Gmail tab.',
    repairNotGmailNext: 'Switch to a mail.google.com tab, then click Repair Current Gmail Tab.',
    repairStarted: 'Reinjecting the latest content script into the current Gmail tab...',
    repairSuccess: 'Repair complete. Reopen the AI bubble in Gmail and try scan or reply generation again.',
    repairFailed: 'Repair failed',
    runtimeTitle: 'Runtime Mode',
    modeLabel: 'Mode',
    minConfidenceLabel: 'Min confidence for auto-send',
    autoScanMinutesLabel: 'Auto scan interval minutes',
    autoScanMinutesHelp: 'Default is 30 minutes. Minimum is 5 minutes.',
    blockSensitiveLabel: 'Block sensitive emails by default',
    enableAuditLabel: 'Store local audit metadata',
    replyIdentityModeLabel: 'Reply sender name',
    replyIdentityUserOption: 'Use my name',
    replyIdentityAssistantOption: 'Use AI assistant name',
    userDisplayNameLabel: 'My name',
    assistantDisplayNameLabel: 'AI assistant name',
    webLlmEnabledLabel: 'Use WebLLM local model',
    webLlmModelLabel: 'WebLLM model id',
    webLlmTemperatureLabel: 'WebLLM temperature',
    webLlmMaxTokensLabel: 'WebLLM max tokens',
    webLlmReady: 'WebLLM ready',
    webLlmLoading: 'WebLLM loading',
    webLlmUnavailable: 'WebLLM unavailable',
    webLlmStarting: 'Starting WebLLM...',
    saveSettings: 'Save Settings',
    addRuleTitle: 'Add Rule',
    editRuleTitle: 'Edit Rule',
    ruleFormModeCreate: 'Create new rule',
    ruleFormModeEdit: 'Editing existing rule',
    ruleFormIdle: 'Create a new rule or select an existing rule to edit.',
    ruleSaved: 'Rule saved.',
    ruleUpdated: 'Rule updated.',
    ruleEditCancelled: 'Edit cancelled.',
    ruleNameHelp: 'Use a short name that helps you identify this rule later.',
    rulePriorityHelp: 'Higher numbers win when multiple rules match the same thread.',
    ruleSenderHelp: 'Match full addresses or partial domains, separated by commas.',
    ruleSubjectRegexHelp: 'Optional regular expression for the Gmail subject line.',
    ruleContainsHelp: 'All listed keywords must appear in the message body.',
    ruleToneHelp: 'Reference a local tone preset, for example professional or concise.',
    ruleToneExamples: 'Examples: professional, concise, friendly, formal.',
    ruleInstructionHelp: 'Describe how the assistant should reply when this rule matches.',
    ruleActionHelp: 'Choose whether this rule only suggests text, creates a draft, or allows guarded auto-send.',
    ruleActionExamples: 'manual = suggest only, draft_assist = create draft, guarded_autosend = may auto-send when safe.',
    ruleActionManualLabel: 'manual: suggest text only',
    ruleActionDraftAssistLabel: 'draft_assist: create Gmail draft',
    ruleActionGuardedAutosendLabel: 'guarded_autosend: allow guarded auto-send',
    ruleConfidenceHelp: 'Guarded auto-send only applies when confidence meets or exceeds this threshold.',
    ruleNameLabel: 'Rule name',
    rulePriorityLabel: 'Priority',
    ruleSenderLabel: 'Sender matcher (csv)',
    ruleSubjectRegexLabel: 'Subject regex',
    ruleContainsLabel: 'Body contains (csv-all)',
    ruleToneLabel: 'Tone profile id',
    ruleInstructionLabel: 'Instruction template',
    ruleActionLabel: 'Allowed action',
    ruleConfidenceLabel: 'Auto-send confidence threshold',
    addRule: 'Add Rule',
    saveRule: 'Save Rule',
    editRule: 'Edit',
    cancelEditRule: 'Cancel Edit',
    rulesTitle: 'Rules',
    noRules: 'No rules yet.',
    toggleEnable: 'Enable',
    toggleDisable: 'Disable',
    delete: 'Delete',
    priorityMeta: 'priority',
    actionMeta: 'action',
    senderMeta: 'sender',
    subjectRegexMeta: 'subject regex',
    containsMeta: 'contains',
    auditTitle: 'Local Audit (last 30)',
    noAudit: 'No audit records.',
    confidence: 'confidence',
    unknown: 'unknown',
    noSubject: '(no subject)',
    unnamedRule: 'Unnamed Rule',
    panelInitFailed: 'Panel init failed',
    placeholderRuleName: 'e.g. Vendor follow-up',
    placeholderRuleSender: 'example.com,alice@',
    placeholderRuleSubjectRegex: 'invoice|follow up',
    placeholderRuleContains: 'payment,deadline',
    placeholderRuleTone: 'professional / concise',
    placeholderRuleInstruction: 'How should the assistant reply?',
    issueSidePanelApi: 'Side Panel API is unavailable in this browser/runtime.',
    issueWebGpu: 'WebGPU is unavailable; WebLLM cannot run local models in this browser profile.',
    issueMissingPermission: 'Permission missing or not granted',
    permissionNames: {
      storage: 'storage',
      activeTab: 'activeTab',
      scripting: 'scripting',
      alarms: 'alarms',
      sidePanel: 'sidePanel',
      gmailHost: 'Gmail host permission (https://mail.google.com/*)',
    },
  },
  'zh-TW': {
    titleMain: 'AI Mail Butler 本地端',
    titleSub: '所有規則與審計資料都儲存在此瀏覽器設定檔。',
    langLabel: '語言',
    followWebLangLabel: '跟隨網站語系',
    syncLangNow: '立即同步',
    syncIdle: '語系同步會讀取目前作用中網站分頁的 i18n_lang。',
    syncSuccess: '已從目前分頁同步語系',
    syncSuccessOrigin: '來源 origin',
    syncNoValue: '目前分頁 localStorage 找不到 i18n_lang。',
    syncFailed: '語系同步失敗',
    syncManualHint: '請先切到 AI Mail Butler 網頁分頁，再按「立即同步」。',
    syncPermissionDenied: '目前這個分頁無法讓 Extension 存取頁面內容。',
    syncPermissionNext: '請先切到 AI Mail Butler 網頁分頁，從該分頁點一次 Extension 圖示後，再按「立即同步」。如果你目前在其他網站，建議改用手動選擇語系。',
    syncTabUnsupported: '目前分頁不支援語系同步。',
    syncWhitelistRejected: '目前分頁的 origin 不在 Web App 白名單內。',
    syncWhitelistNext: '請先把這個 origin 加到下方白名單，再切回該網站分頁重新同步。',
    webWhitelistTitle: 'Web App Origin 白名單',
    webWhitelistHelp: '只有白名單內的 origin 可以用來同步網站語系。',
    webWhitelistPlaceholder: 'https://app.example.com',
    addWhitelistOrigin: '新增',
    noWhitelistOrigins: '目前沒有白名單 origin。',
    removeWhitelistOrigin: '移除',
    whitelistSaved: '白名單已更新。',
    whitelistInvalid: '請輸入有效的 origin，例如 https://app.example.com',
    whitelistDuplicate: '這個 origin 已經在白名單裡。',
    whitelistCurrentOrigin: '目前作用中 origin',
    capabilityTitle: '權限與能力檢查',
    refresh: '重新檢查',
    capabilityChecking: '正在檢查執行環境能力...',
    capabilityReadyTitle: '可用',
    capabilityReadyDesc: '必要 API 與權限皆可使用。',
    capabilityActionTitle: '需要處理',
    capabilityActionDesc: '部分權限或 API 不可用，某些自動化功能會受限。',
    checkedAt: '檢查時間',
    repairGmailTab: '修復目前 Gmail 分頁',
    repairIdle: '如果你剛重新載入 Extension，導致 Gmail 動作失敗，可以先修復目前 Gmail 分頁。',
    repairNotGmail: '目前作用中的分頁不是 Gmail。',
    repairNotGmailNext: '請先切換到 mail.google.com 分頁，再按「修復目前 Gmail 分頁」。',
    repairStarted: '正在重新注入最新的 content script 到目前 Gmail 分頁...',
    repairSuccess: '修復完成。請回到 Gmail 重新打開 AI 氣泡，再測試掃描或產生回覆。',
    repairFailed: '修復失敗',
    runtimeTitle: '執行模式',
    modeLabel: '模式',
    minConfidenceLabel: '自動寄送最低信心分數',
    autoScanMinutesLabel: '自動掃描間隔（分鐘）',
    autoScanMinutesHelp: '預設 30 分鐘。最短 5 分鐘。',
    blockSensitiveLabel: '預設封鎖敏感郵件自動寄送',
    enableAuditLabel: '儲存本地審計摘要',
    replyIdentityModeLabel: '回信署名',
    replyIdentityUserOption: '使用我的名字',
    replyIdentityAssistantOption: '使用 AI 電子郵件助理的名字',
    userDisplayNameLabel: '我的名字',
    assistantDisplayNameLabel: 'AI 電子郵件助理名字',
    webLlmEnabledLabel: '使用 WebLLM 本地模型',
    webLlmModelLabel: 'WebLLM 模型 ID',
    webLlmTemperatureLabel: 'WebLLM temperature',
    webLlmMaxTokensLabel: 'WebLLM max tokens',
    webLlmReady: 'WebLLM 已就緒',
    webLlmLoading: 'WebLLM 載入中',
    webLlmUnavailable: 'WebLLM 不可用',
    webLlmStarting: '正在啟動 WebLLM...',
    saveSettings: '儲存設定',
    addRuleTitle: '新增規則',
    editRuleTitle: '編輯規則',
    ruleFormModeCreate: '建立新規則',
    ruleFormModeEdit: '正在編輯既有規則',
    ruleFormIdle: '你可以建立新規則，或選擇既有規則進行編輯。',
    ruleSaved: '規則已儲存。',
    ruleUpdated: '規則已更新。',
    ruleEditCancelled: '已取消編輯。',
    ruleNameHelp: '使用簡短名稱，方便之後辨識這條規則。',
    rulePriorityHelp: '當多條規則同時命中時，數字較高者優先。',
    ruleSenderHelp: '可填完整信箱或網域片段，使用逗號分隔。',
    ruleSubjectRegexHelp: '可選填 Gmail 主旨的正規表示式。',
    ruleContainsHelp: '列出的所有關鍵字都必須出現在信件內容中。',
    ruleToneHelp: '請輸入語氣 ID：professional（專業穩重）、concise（精簡直接）、friendly（親切自然）、formal（正式禮貌）。',
    ruleToneExamples: '語氣 ID 會影響回覆措辭與長度；例如 concise 會短一點，formal 會更有禮貌和距離感。',
    ruleInstructionHelp: '描述這條規則命中時，AI 應如何回覆。',
    ruleActionHelp: '選擇此規則只建議文字、建立草稿，或允許條件式自動寄送。',
    ruleActionExamples: 'manual = 只建議文字，draft_assist = 建立 Gmail 草稿，guarded_autosend = 在安全條件下可自動寄送。',
    ruleActionManualLabel: 'manual：只建議文字',
    ruleActionDraftAssistLabel: 'draft_assist：建立 Gmail 草稿',
    ruleActionGuardedAutosendLabel: 'guarded_autosend：允許條件式自動寄送',
    ruleConfidenceHelp: '只有在信心分數達到此門檻時，才允許 guarded auto-send。',
    ruleNameLabel: '規則名稱',
    rulePriorityLabel: '優先順序',
    ruleSenderLabel: '寄件者匹配（csv）',
    ruleSubjectRegexLabel: '主旨 regex',
    ruleContainsLabel: '內容必含（csv-all）',
    ruleToneLabel: '語氣設定 ID',
    ruleInstructionLabel: '指令模板',
    ruleActionLabel: '允許動作',
    ruleConfidenceLabel: '自動寄送信心門檻',
    addRule: '新增規則',
    saveRule: '儲存規則',
    editRule: '編輯',
    cancelEditRule: '取消編輯',
    rulesTitle: '規則清單',
    noRules: '目前沒有規則。',
    toggleEnable: '啟用',
    toggleDisable: '停用',
    delete: '刪除',
    priorityMeta: '優先順序',
    actionMeta: '動作',
    senderMeta: '寄件者',
    subjectRegexMeta: '主旨 regex',
    containsMeta: '內容必含',
    auditTitle: '本地審計（最近 30 筆）',
    noAudit: '目前沒有審計紀錄。',
    confidence: '信心分數',
    unknown: '未知',
    noSubject: '（無主旨）',
    unnamedRule: '未命名規則',
    panelInitFailed: '面板初始化失敗',
    placeholderRuleName: '例如：供應商追蹤回覆',
    placeholderRuleSender: 'example.com,alice@',
    placeholderRuleSubjectRegex: 'invoice|follow up',
    placeholderRuleContains: 'payment,deadline',
    placeholderRuleTone: 'professional / concise',
    placeholderRuleInstruction: '請描述要如何回覆此類郵件',
    issueSidePanelApi: '此瀏覽器或執行環境不支援 Side Panel API。',
    issueWebGpu: '此瀏覽器設定檔不支援 WebGPU，WebLLM 無法執行本地模型。',
    issueMissingPermission: '缺少或未授權權限',
    permissionNames: {
      storage: 'storage',
      activeTab: 'activeTab',
      scripting: 'scripting',
      alarms: 'alarms',
      sidePanel: 'sidePanel',
      gmailHost: 'Gmail 網域權限（https://mail.google.com/*）',
    },
  },
};

let currentLang = 'en';
let webWhitelist = [];
let editingRuleId = null;

function byId(id) {
  return document.getElementById(id);
}

function htmlEscape(text) {
  return String(text)
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#39;');
}

function normalizeLang(input) {
  return input === 'zh-TW' ? 'zh-TW' : 'en';
}

function t(key) {
  return I18N[currentLang]?.[key] ?? I18N.en[key] ?? key;
}

function tPermission(name) {
  return I18N[currentLang]?.permissionNames?.[name] ?? name;
}

function formatIssue(issue) {
  if (issue === 'Side Panel API is unavailable in this browser/runtime.') {
    return t('issueSidePanelApi');
  }

  if (issue === 'WebGPU is unavailable; WebLLM cannot run local models in this browser profile.') {
    return t('issueWebGpu');
  }

  const prefix = 'Permission missing or not granted: ';
  if (String(issue).startsWith(prefix)) {
    const name = String(issue).slice(prefix.length).trim();
    return `${t('issueMissingPermission')}: ${tPermission(name)}`;
  }

  return issue;
}

function detectBrowserLanguage() {
  const browserLang = navigator.language || '';
  return browserLang.toLowerCase().startsWith('zh') ? 'zh-TW' : 'en';
}

async function loadLangPreference() {
  const data = await chrome.storage.local.get([LANG_KEY]);
  return normalizeLang(data[LANG_KEY] || detectBrowserLanguage());
}

async function saveLangPreference(lang) {
  await chrome.storage.local.set({ [LANG_KEY]: normalizeLang(lang) });
}

async function loadFollowWebLangPreference() {
  const data = await chrome.storage.local.get([FOLLOW_WEBAPP_LANG_KEY]);
  return !!data[FOLLOW_WEBAPP_LANG_KEY];
}

async function saveFollowWebLangPreference(enabled) {
  await chrome.storage.local.set({ [FOLLOW_WEBAPP_LANG_KEY]: !!enabled });
}

async function loadWebAppWhitelist() {
  const data = await chrome.storage.local.get([WEBAPP_LANG_WHITELIST_KEY]);
  const saved = Array.isArray(data[WEBAPP_LANG_WHITELIST_KEY]) ? data[WEBAPP_LANG_WHITELIST_KEY] : DEFAULT_WEBAPP_LANG_WHITELIST;
  return Array.from(new Set(saved.map(normalizeOrigin).filter(Boolean)));
}

async function saveWebAppWhitelist(origins) {
  const next = Array.from(new Set(origins.map(normalizeOrigin).filter(Boolean)));
  await chrome.storage.local.set({ [WEBAPP_LANG_WHITELIST_KEY]: next });
}

function normalizeOrigin(input) {
  if (!input) return null;
  try {
    return new URL(String(input).trim()).origin;
  } catch {
    return null;
  }
}

async function getActiveTabInfo() {
  const tabs = await chrome.tabs.query({ active: true, lastFocusedWindow: true });
  const active = tabs.find((t) => !!t.id);
  if (!active?.id) return { ok: false, reason: 'NO_ACTIVE_TAB' };
  const origin = active.url ? normalizeOrigin(active.url) : null;
  return { ok: true, tabId: active.id, url: active.url || '', origin };
}

function isGmailUrl(url) {
  return typeof url === 'string' && /^https:\/\/mail\.google\.com\//.test(url);
}

async function readWebAppLanguageFromActiveTab() {
  const tabInfo = await getActiveTabInfo();
  if (!tabInfo.ok) {
    return { ok: false, reason: 'NO_ACTIVE_TAB' };
  }

  if (!tabInfo.origin) {
    return { ok: false, reason: 'TAB_UNSUPPORTED' };
  }

  if (!webWhitelist.includes(tabInfo.origin)) {
    return { ok: false, reason: 'WHITELIST_REJECTED', origin: tabInfo.origin };
  }

  try {
    const injected = await chrome.scripting.executeScript({
      target: { tabId: tabInfo.tabId },
      func: () => {
        const value = localStorage.getItem('i18n_lang');
        return value || null;
      },
    });

    const raw = injected?.[0]?.result;
    if (!raw) return { ok: false, reason: 'NO_LANG' };
    const normalized = raw === 'zh-TW' ? 'zh-TW' : raw === 'en' ? 'en' : null;
    if (!normalized) return { ok: false, reason: 'NO_LANG' };
    return { ok: true, lang: normalized, origin: tabInfo.origin };
  } catch (err) {
    const reason = String(err?.message || err);
    if (reason.includes('Cannot access contents of the page') || reason.includes('host permission')) {
      return { ok: false, reason: 'PERMISSION_DENIED' };
    }
    if (reason.includes('The extensions gallery cannot be scripted') || reason.includes('Cannot access a chrome://')) {
      return { ok: false, reason: 'TAB_UNSUPPORTED' };
    }
    return { ok: false, reason };
  }
}

async function getState() {
  const res = await chrome.runtime.sendMessage({ type: 'GET_STATE' });
  if (!res?.ok) throw new Error(res?.error || 'Failed to get state');
  return res.state;
}

async function saveSettings(settings) {
  const res = await chrome.runtime.sendMessage({ type: 'SAVE_SETTINGS', settings });
  if (!res?.ok) throw new Error(res?.error || 'Failed to save settings');
}

async function saveRules(rules) {
  const res = await chrome.runtime.sendMessage({ type: 'SAVE_RULES', rules });
  if (!res?.ok) throw new Error(res?.error || 'Failed to save rules');
}

async function getCapabilityStatus() {
  const res = await chrome.runtime.sendMessage({ type: 'GET_CAPABILITY_STATUS' });
  if (!res?.ok) throw new Error(res?.error || 'Failed to get capability status');
  return res.status;
}

async function getWebLlmStatus() {
  const res = await chrome.runtime.sendMessage({ type: 'GET_WEBLLM_STATUS' });
  if (!res?.ok) throw new Error(res?.error || 'Failed to get WebLLM status');
  return res.status;
}

async function warmUpWebLlm() {
  const res = await chrome.runtime.sendMessage({ type: 'WARM_UP_WEBLLM' });
  if (!res?.ok) throw new Error(res?.error || 'Failed to warm up WebLLM');
  return res.status;
}

function renderRepairStatus(message, isError = false) {
  const el = byId('repairGmailStatus');
  if (!el) return;
  el.textContent = message;
  el.style.color = isError ? '#b42318' : '';
}

async function repairCurrentGmailTab() {
  const tabInfo = await getActiveTabInfo();
  if (!tabInfo.ok || !isGmailUrl(tabInfo.url)) {
    renderRepairStatus(`${t('repairNotGmail')} ${t('repairNotGmailNext')}`, true);
    return false;
  }

  renderRepairStatus(t('repairStarted'));
  try {
    await chrome.scripting.executeScript({
      target: { tabId: tabInfo.tabId },
      files: ['content-script.js'],
    });
    renderRepairStatus(t('repairSuccess'));
    return true;
  } catch (err) {
    renderRepairStatus(`${t('repairFailed')}: ${String(err?.message || err)}`, true);
    return false;
  }
}

function applyStaticTranslations() {
  byId('titleMain').textContent = t('titleMain');
  byId('titleSub').textContent = t('titleSub');
  byId('langLabel').textContent = t('langLabel');
  byId('followWebLangLabel').textContent = t('followWebLangLabel');
  byId('syncLangNow').textContent = t('syncLangNow');
  byId('webWhitelistTitle').textContent = t('webWhitelistTitle');
  byId('webWhitelistHelp').textContent = t('webWhitelistHelp');
  byId('webWhitelistInput').placeholder = t('webWhitelistPlaceholder');
  byId('addWhitelistOrigin').textContent = t('addWhitelistOrigin');
  byId('capabilityTitle').textContent = t('capabilityTitle');
  byId('refreshCapability').textContent = t('refresh');
  byId('repairGmailTab').textContent = t('repairGmailTab');

  byId('runtimeTitle').textContent = t('runtimeTitle');
  byId('modeLabel').textContent = t('modeLabel');
  byId('minConfidenceLabel').textContent = t('minConfidenceLabel');
  byId('autoScanMinutesLabel').textContent = t('autoScanMinutesLabel');
  byId('autoScanMinutesHelp').textContent = t('autoScanMinutesHelp');
  byId('blockSensitiveLabel').textContent = t('blockSensitiveLabel');
  byId('enableAuditLabel').textContent = t('enableAuditLabel');
  byId('replyIdentityModeLabel').textContent = t('replyIdentityModeLabel');
  byId('replyIdentityUserOption').textContent = t('replyIdentityUserOption');
  byId('replyIdentityAssistantOption').textContent = t('replyIdentityAssistantOption');
  byId('userDisplayNameLabel').textContent = t('userDisplayNameLabel');
  byId('assistantDisplayNameLabel').textContent = t('assistantDisplayNameLabel');
  byId('webLlmEnabledLabel').textContent = t('webLlmEnabledLabel');
  byId('webLlmModelLabel').textContent = t('webLlmModelLabel');
  byId('webLlmTemperatureLabel').textContent = t('webLlmTemperatureLabel');
  byId('webLlmMaxTokensLabel').textContent = t('webLlmMaxTokensLabel');
  byId('saveSettings').textContent = t('saveSettings');

  byId('addRuleTitle').textContent = editingRuleId ? t('editRuleTitle') : t('addRuleTitle');
  byId('ruleFormMode').textContent = editingRuleId ? t('ruleFormModeEdit') : t('ruleFormModeCreate');
  byId('cancelEditRule').textContent = t('cancelEditRule');
  byId('ruleNameLabel').textContent = t('ruleNameLabel');
  byId('ruleNameHelp').textContent = t('ruleNameHelp');
  byId('rulePriorityLabel').textContent = t('rulePriorityLabel');
  byId('rulePriorityHelp').textContent = t('rulePriorityHelp');
  byId('ruleSenderLabel').textContent = t('ruleSenderLabel');
  byId('ruleSenderHelp').textContent = t('ruleSenderHelp');
  byId('ruleSubjectRegexLabel').textContent = t('ruleSubjectRegexLabel');
  byId('ruleSubjectRegexHelp').textContent = t('ruleSubjectRegexHelp');
  byId('ruleContainsLabel').textContent = t('ruleContainsLabel');
  byId('ruleContainsHelp').textContent = t('ruleContainsHelp');
  byId('ruleToneLabel').textContent = t('ruleToneLabel');
  byId('ruleToneHelp').textContent = t('ruleToneHelp');
  byId('ruleToneExamples').textContent = t('ruleToneExamples');
  byId('ruleInstructionLabel').textContent = t('ruleInstructionLabel');
  byId('ruleInstructionHelp').textContent = t('ruleInstructionHelp');
  byId('ruleActionLabel').textContent = t('ruleActionLabel');
  byId('ruleActionHelp').textContent = t('ruleActionHelp');
  byId('ruleActionExamples').textContent = t('ruleActionExamples');
  byId('ruleActionOptionManual').textContent = t('ruleActionManualLabel');
  byId('ruleActionOptionDraftAssist').textContent = t('ruleActionDraftAssistLabel');
  byId('ruleActionOptionGuardedAutosend').textContent = t('ruleActionGuardedAutosendLabel');
  byId('ruleConfidenceLabel').textContent = t('ruleConfidenceLabel');
  byId('ruleConfidenceHelp').textContent = t('ruleConfidenceHelp');
  byId('addRule').textContent = editingRuleId ? t('saveRule') : t('addRule');

  byId('rulesTitle').textContent = t('rulesTitle');
  byId('auditTitle').textContent = t('auditTitle');

  byId('ruleName').placeholder = t('placeholderRuleName');
  byId('ruleSender').placeholder = t('placeholderRuleSender');
  byId('ruleSubjectRegex').placeholder = t('placeholderRuleSubjectRegex');
  byId('ruleContains').placeholder = t('placeholderRuleContains');
  byId('ruleTone').placeholder = t('placeholderRuleTone');
  byId('ruleInstruction').placeholder = t('placeholderRuleInstruction');

  if (byId('capabilityStatus').className === 'meta') {
    byId('capabilityStatus').textContent = t('capabilityChecking');
  }

  if (!byId('langSyncStatus').textContent?.trim()) {
    byId('langSyncStatus').textContent = t('syncIdle');
  }

  if (!byId('repairGmailStatus').textContent?.trim()) {
    byId('repairGmailStatus').textContent = t('repairIdle');
  }

  if (!byId('ruleFormStatus').textContent?.trim()) {
    byId('ruleFormStatus').textContent = t('ruleFormIdle');
  }
}

function renderLangSyncStatus(message, isError = false) {
  const el = byId('langSyncStatus');
  if (!el) return;
  el.textContent = message;
  el.style.color = isError ? '#b42318' : '';
}

function renderWhitelistStatus(message, isError = false) {
  const el = byId('webWhitelistStatus');
  if (!el) return;
  el.textContent = message;
  el.style.color = isError ? '#b42318' : '';
}

function renderRuleFormStatus(message, isError = false) {
  const el = byId('ruleFormStatus');
  if (!el) return;
  el.textContent = message;
  el.style.color = isError ? '#b42318' : '';
}

function renderWhitelist(origins) {
  const el = byId('webWhitelistList');
  if (!el) return;

  if (!origins.length) {
    el.innerHTML = `<div class="meta">${htmlEscape(t('noWhitelistOrigins'))}</div>`;
    return;
  }

  el.innerHTML = `
    <div class="chip-row">
      ${origins
        .map(
          (origin) => `
            <div class="chip">
              <span>${htmlEscape(origin)}</span>
              <button class="secondary" data-action="remove-whitelist" data-origin="${htmlEscape(origin)}" type="button">${htmlEscape(t('removeWhitelistOrigin'))}</button>
            </div>
          `,
        )
        .join('')}
    </div>
  `;
}

async function addWhitelistOriginFromInput() {
  const input = byId('webWhitelistInput');
  const normalized = normalizeOrigin(input?.value || '');
  if (!normalized) {
    renderWhitelistStatus(t('whitelistInvalid'), true);
    return;
  }
  if (webWhitelist.includes(normalized)) {
    renderWhitelistStatus(t('whitelistDuplicate'), true);
    return;
  }

  webWhitelist = [...webWhitelist, normalized];
  await saveWebAppWhitelist(webWhitelist);
  renderWhitelist(webWhitelist);
  renderWhitelistStatus(t('whitelistSaved'));
  input.value = '';
}

async function syncLanguageFromActiveTab() {
  const res = await readWebAppLanguageFromActiveTab();
  if (!res.ok) {
    if (res.reason === 'WHITELIST_REJECTED') {
      renderLangSyncStatus(`${t('syncWhitelistRejected')} ${t('syncWhitelistNext')} ${t('whitelistCurrentOrigin')}: ${res.origin || '-'}`, true);
      return false;
    }
    if (res.reason === 'NO_LANG') {
      renderLangSyncStatus(`${t('syncNoValue')} ${t('syncManualHint')}`, true);
      return false;
    }
    if (res.reason === 'PERMISSION_DENIED') {
      renderLangSyncStatus(`${t('syncPermissionDenied')} ${t('syncPermissionNext')}`, true);
      return false;
    }
    if (res.reason === 'TAB_UNSUPPORTED') {
      renderLangSyncStatus(`${t('syncTabUnsupported')} ${t('syncManualHint')}`, true);
      return false;
    }
    renderLangSyncStatus(`${t('syncFailed')}: ${res.reason}`, true);
    return false;
  }

  currentLang = normalizeLang(res.lang);
  await saveLangPreference(currentLang);
  byId('langSelect').value = currentLang;
  applyStaticTranslations();
  renderLangSyncStatus(`${t('syncSuccess')}: ${currentLang} | ${t('syncSuccessOrigin')}: ${res.origin || '-'}`);
  return true;
}

function fillRuleForm(rule) {
  byId('ruleName').value = rule?.name || '';
  byId('rulePriority').value = String(rule?.priority ?? 10);
  byId('ruleSender').value = rule?.match?.sender || '';
  byId('ruleSubjectRegex').value = rule?.match?.subject_regex || '';
  byId('ruleContains').value = rule?.match?.contains || '';
  byId('ruleTone').value = rule?.tone_profile_id || '';
  byId('ruleInstruction').value = rule?.instruction_template || '';
  byId('ruleAction').value = rule?.allowed_actions || 'draft_assist';
  byId('ruleConfidence').value = String(rule?.min_confidence_auto_send ?? 0.9);
}

function startEditingRule(rule) {
  editingRuleId = rule.id;
  fillRuleForm(rule);
  applyStaticTranslations();
  renderRuleFormStatus(t('ruleFormModeEdit'));
}

function stopEditingRule(showStatus = true) {
  editingRuleId = null;
  resetRuleForm();
  applyStaticTranslations();
  renderRuleFormStatus(showStatus ? t('ruleEditCancelled') : t('ruleFormIdle'));
}

function collectRuleFromForm() {
  return {
    id: editingRuleId || crypto.randomUUID(),
    enabled: true,
    name: byId('ruleName').value.trim() || t('unnamedRule'),
    priority: Number(byId('rulePriority').value || 0),
    match: {
      sender: byId('ruleSender').value.trim(),
      subject_regex: byId('ruleSubjectRegex').value.trim(),
      contains: byId('ruleContains').value.trim(),
    },
    tone_profile_id: byId('ruleTone').value.trim() || 'professional',
    instruction_template: byId('ruleInstruction').value.trim(),
    allowed_actions: byId('ruleAction').value,
    min_confidence_auto_send: Number(byId('ruleConfidence').value || 0.9),
  };
}

function renderRules(rules) {
  const container = byId('ruleList');
  if (!rules.length) {
    container.innerHTML = `<div class="meta">${htmlEscape(t('noRules'))}</div>`;
    return;
  }

  container.innerHTML = rules
    .map(
      (r) => `
      <div class="rule-item" data-rule-id="${htmlEscape(r.id)}">
        <div class="rule-item-head">
          <strong>${htmlEscape(r.name || r.id)}</strong>
          <div>
            <button class="secondary" data-action="edit" data-id="${htmlEscape(r.id)}">${htmlEscape(t('editRule'))}</button>
            <button class="secondary" data-action="toggle" data-id="${htmlEscape(r.id)}">${r.enabled ? htmlEscape(t('toggleDisable')) : htmlEscape(t('toggleEnable'))}</button>
            <button class="secondary" data-action="remove" data-id="${htmlEscape(r.id)}">${htmlEscape(t('delete'))}</button>
          </div>
        </div>
        <div class="meta">${htmlEscape(t('priorityMeta'))}: ${Number(r.priority || 0)} | ${htmlEscape(t('actionMeta'))}: ${htmlEscape(r.allowed_actions || 'draft_assist')}</div>
        <div class="meta">${htmlEscape(t('senderMeta'))}: ${htmlEscape(r.match?.sender || '-')}</div>
        <div class="meta">${htmlEscape(t('subjectRegexMeta'))}: ${htmlEscape(r.match?.subject_regex || '-')}</div>
        <div class="meta">${htmlEscape(t('containsMeta'))}: ${htmlEscape(r.match?.contains || '-')}</div>
      </div>
    `,
    )
    .join('');
}

function renderAudit(logs) {
  const container = byId('auditList');
  const top = logs.slice(0, 30);
  if (!top.length) {
    container.innerHTML = `<div class="meta">${htmlEscape(t('noAudit'))}</div>`;
    return;
  }

  container.innerHTML = top
    .map(
      (item) => `
      <div class="audit-item">
        <div><strong>${htmlEscape(item.action || t('unknown'))}</strong> | ${htmlEscape(t('confidence'))} ${Number(item.confidence || 0).toFixed(2)}</div>
        <div class="meta">${htmlEscape(item.subject || t('noSubject'))}</div>
        <div class="meta">${htmlEscape(t('senderMeta'))}: ${htmlEscape(item.sender || '-')}</div>
        <div class="meta">${htmlEscape(item.ts || '-')}</div>
      </div>
    `,
    )
    .join('');
}

function fillSettings(settings) {
  byId('mode').value = settings.mode || 'draft_assist';
  byId('minConfidence').value = String(settings.minConfidenceAutoSend ?? 0.9);
  byId('autoScanMinutes').value = String(Math.max(5, Number(settings.autoScanMinutes ?? 30)));
  byId('blockSensitive').checked = !!settings.blockSensitiveByDefault;
  byId('enableAudit').checked = !!settings.localAuditEnabled;
  byId('replyIdentityMode').value = settings.replyIdentityMode === 'user' ? 'user' : 'assistant';
  byId('userDisplayName').value = settings.userDisplayName || '';
  byId('assistantDisplayName').value = settings.assistantDisplayName || 'AI Mail Butler';
  byId('webLlmEnabled').checked = settings.webLlmEnabled !== false;
  byId('webLlmModelId').value = settings.webLlmModelId || 'Llama-3.2-1B-Instruct-q4f16_1-MLC';
  byId('webLlmTemperature').value = String(settings.webLlmTemperature ?? 0.3);
  byId('webLlmMaxTokens').value = String(settings.webLlmMaxTokens ?? 700);
}

function renderWebLlmStatus(status) {
  const container = byId('webLlmStatus');
  if (!container) return;

  const label = status?.ready ? t('webLlmReady') : status?.available ? t('webLlmLoading') : t('webLlmUnavailable');
  const detail = [status?.modelId, status?.progressText, status?.error].filter(Boolean).join(' | ');
  container.textContent = `${label}: ${detail || '-'}`;
  container.style.color = status?.ready || status?.available ? '' : '#b42318';
}

async function warmUpAndTrackWebLlmStatus() {
  renderWebLlmStatus({
    available: true,
    ready: false,
    modelId: byId('webLlmModelId')?.value || '',
    progressText: t('webLlmStarting'),
    error: '',
  });

  if (chrome?.runtime?.connect) {
    return new Promise((resolve, reject) => {
      let settled = false;
      let latestStatus = null;
      const timeoutId = setTimeout(() => {
        if (settled) return;
        settled = true;
        reject(new Error('WebLLM warm-up timed out.'));
      }, 15 * 60 * 1000);

      const port = chrome.runtime.connect({ name: 'webllm-warmup' });
      port.onMessage.addListener((message) => {
        if (message?.type !== 'WEBLLM_STATUS') return;
        latestStatus = message.status;
        renderWebLlmStatus(latestStatus);

        if (latestStatus?.ready || latestStatus?.error || latestStatus?.progressText?.includes('disabled')) {
          settled = true;
          clearTimeout(timeoutId);
          resolve(latestStatus);
          try {
            port.disconnect();
          } catch {
            // The background worker may close the port first.
          }
        }
      });
      port.onDisconnect.addListener(() => {
        clearTimeout(timeoutId);
        if (settled) return;
        settled = true;
        if (latestStatus) {
          resolve(latestStatus);
          return;
        }
        getWebLlmStatus().then(resolve).catch(reject);
      });
      port.postMessage({ type: 'START_WEBLLM_WARMUP' });
    });
  }

  const status = await warmUpWebLlm();
  renderWebLlmStatus(status);
  return status;
}

function resetRuleForm() {
  byId('ruleName').value = '';
  byId('rulePriority').value = '10';
  byId('ruleSender').value = '';
  byId('ruleSubjectRegex').value = '';
  byId('ruleContains').value = '';
  byId('ruleTone').value = '';
  byId('ruleInstruction').value = '';
  byId('ruleAction').value = 'draft_assist';
  byId('ruleConfidence').value = '0.9';
}

function renderCapabilityStatus(status) {
  const container = byId('capabilityStatus');
  if (!container) return;

  const checkedAt = status?.checkedAt ? new Date(status.checkedAt).toLocaleString() : '-';
  const issues = Array.isArray(status?.issues) ? status.issues : [];

  if (!issues.length) {
    container.className = 'status-ok';
    container.innerHTML = [
      `<strong>${htmlEscape(t('capabilityReadyTitle'))}</strong>: ${htmlEscape(t('capabilityReadyDesc'))}`,
      `<div class="meta">${htmlEscape(t('checkedAt'))}: ${htmlEscape(checkedAt)}</div>`,
    ].join('');
    return;
  }

  container.className = 'status-warn';
  container.innerHTML = [
    `<strong>${htmlEscape(t('capabilityActionTitle'))}</strong>: ${htmlEscape(t('capabilityActionDesc'))}`,
    `<div class="meta">${htmlEscape(t('checkedAt'))}: ${htmlEscape(checkedAt)}</div>`,
    `<ul class="status-list">${issues.map((i) => `<li>${htmlEscape(formatIssue(i))}</li>`).join('')}</ul>`,
  ].join('');
}

async function init() {
  let state = await getState();
  let capability = await getCapabilityStatus();
  let webLlmStatus = await getWebLlmStatus();

  currentLang = await loadLangPreference();
  webWhitelist = await loadWebAppWhitelist();
  const followWebLang = await loadFollowWebLangPreference();
  byId('langSelect').value = currentLang;
  byId('followWebLang').checked = followWebLang;

  applyStaticTranslations();
  renderLangSyncStatus(t('syncIdle'));
  renderWhitelist(webWhitelist);
  renderWhitelistStatus('');
  renderRepairStatus(t('repairIdle'));
  renderRuleFormStatus(t('ruleFormIdle'));
  fillSettings(state.settings);
  renderWebLlmStatus(webLlmStatus);
  renderRules(state.rules);
  renderAudit(state.auditLogs);
  renderCapabilityStatus(capability);
  warmUpAndTrackWebLlmStatus()
    .then((status) => {
      webLlmStatus = status;
    })
    .catch((err) => {
      renderWebLlmStatus({
        available: false,
        ready: false,
        modelId: state.settings.webLlmModelId || '',
        progressText: t('webLlmUnavailable'),
        error: String(err?.message || err),
      });
    });

  if (followWebLang) {
    await syncLanguageFromActiveTab();
    renderRules(state.rules);
    renderAudit(state.auditLogs);
    renderCapabilityStatus(capability);
  }

  byId('langSelect').addEventListener('change', async (event) => {
    const target = event.target;
    if (!(target instanceof HTMLSelectElement)) return;

    currentLang = normalizeLang(target.value);
    await saveLangPreference(currentLang);

    applyStaticTranslations();
    renderLangSyncStatus(t('syncIdle'));
    renderWhitelist(webWhitelist);
    renderRepairStatus(t('repairIdle'));
    renderRules(state.rules);
    renderAudit(state.auditLogs);
    renderCapabilityStatus(capability);
    renderWebLlmStatus(webLlmStatus);
  });

  byId('repairGmailTab').addEventListener('click', async () => {
    await repairCurrentGmailTab();
  });

  byId('cancelEditRule').addEventListener('click', () => {
    stopEditingRule();
  });

  byId('addWhitelistOrigin').addEventListener('click', async () => {
    await addWhitelistOriginFromInput();
  });

  byId('webWhitelistInput').addEventListener('keydown', async (event) => {
    if (event.key !== 'Enter') return;
    event.preventDefault();
    await addWhitelistOriginFromInput();
  });

  byId('webWhitelistList').addEventListener('click', async (event) => {
    const target = event.target;
    if (!(target instanceof HTMLElement)) return;
    const action = target.getAttribute('data-action');
    const origin = target.getAttribute('data-origin');
    if (action !== 'remove-whitelist' || !origin) return;

    webWhitelist = webWhitelist.filter((item) => item !== origin);
    await saveWebAppWhitelist(webWhitelist);
    renderWhitelist(webWhitelist);
    renderWhitelistStatus(t('whitelistSaved'));
  });

  byId('followWebLang').addEventListener('change', async (event) => {
    const target = event.target;
    if (!(target instanceof HTMLInputElement)) return;
    await saveFollowWebLangPreference(target.checked);
    if (target.checked) {
      await syncLanguageFromActiveTab();
      renderRules(state.rules);
      renderAudit(state.auditLogs);
      renderCapabilityStatus(capability);
      webLlmStatus = await getWebLlmStatus();
      renderWebLlmStatus(webLlmStatus);
    } else {
      renderLangSyncStatus(t('syncIdle'));
    }
  });

  byId('syncLangNow').addEventListener('click', async () => {
    await syncLanguageFromActiveTab();
    renderRules(state.rules);
    renderAudit(state.auditLogs);
    renderCapabilityStatus(capability);
    webLlmStatus = await getWebLlmStatus();
    renderWebLlmStatus(webLlmStatus);
  });

  byId('refreshCapability').addEventListener('click', async () => {
    capability = await getCapabilityStatus();
    webLlmStatus = await getWebLlmStatus();
    renderCapabilityStatus(capability);
    renderWebLlmStatus(webLlmStatus);
  });

  byId('saveSettings').addEventListener('click', async () => {
    const settings = {
      mode: byId('mode').value,
      minConfidenceAutoSend: Number(byId('minConfidence').value || 0.9),
      autoScanMinutes: Math.max(5, Number(byId('autoScanMinutes').value || 30)),
      blockSensitiveByDefault: byId('blockSensitive').checked,
      localAuditEnabled: byId('enableAudit').checked,
      replyIdentityMode: byId('replyIdentityMode').value === 'user' ? 'user' : 'assistant',
      userDisplayName: byId('userDisplayName').value.trim(),
      assistantDisplayName: byId('assistantDisplayName').value.trim() || 'AI Mail Butler',
      webLlmEnabled: byId('webLlmEnabled').checked,
      webLlmModelId: byId('webLlmModelId').value.trim() || 'Llama-3.2-1B-Instruct-q4f16_1-MLC',
      webLlmTemperature: Number(byId('webLlmTemperature').value || 0.3),
      webLlmMaxTokens: Number(byId('webLlmMaxTokens').value || 700),
    };
    await saveSettings(settings);
    state = await getState();
    fillSettings(state.settings);
    capability = await getCapabilityStatus();
    renderCapabilityStatus(capability);
    webLlmStatus = await warmUpAndTrackWebLlmStatus();
  });

  byId('addRule').addEventListener('click', async () => {
    const draftedRule = collectRuleFromForm();
    const next = editingRuleId
      ? state.rules.map((rule) => (rule.id === editingRuleId ? { ...rule, ...draftedRule, enabled: rule.enabled } : rule))
      : [draftedRule, ...state.rules];

    await saveRules(next);
    state = await getState();
    renderRules(state.rules);
    renderAudit(state.auditLogs);
    capability = await getCapabilityStatus();
    webLlmStatus = await getWebLlmStatus();
    renderCapabilityStatus(capability);
    renderWebLlmStatus(webLlmStatus);
    const updated = !!editingRuleId;
    editingRuleId = null;
    resetRuleForm();
    applyStaticTranslations();
    renderRuleFormStatus(updated ? t('ruleUpdated') : t('ruleSaved'));
  });

  byId('ruleList').addEventListener('click', async (event) => {
    const target = event.target;
    if (!(target instanceof HTMLElement)) return;

    const action = target.getAttribute('data-action');
    const id = target.getAttribute('data-id');
    if (!action || !id) return;

    if (action === 'edit') {
      const selected = state.rules.find((r) => r.id === id);
      if (!selected) return;
      startEditingRule(selected);
      return;
    }

    if (action === 'remove') {
      const next = state.rules.filter((r) => r.id !== id);
      await saveRules(next);
      if (editingRuleId === id) {
        editingRuleId = null;
        resetRuleForm();
        applyStaticTranslations();
        renderRuleFormStatus(t('ruleEditCancelled'));
      }
    }

    if (action === 'toggle') {
      const next = state.rules.map((r) => (r.id === id ? { ...r, enabled: !r.enabled } : r));
      await saveRules(next);
    }

    state = await getState();
    renderRules(state.rules);
    renderAudit(state.auditLogs);
    capability = await getCapabilityStatus();
    webLlmStatus = await getWebLlmStatus();
    renderCapabilityStatus(capability);
    renderWebLlmStatus(webLlmStatus);
  });
}

init().catch((err) => {
  const msg = document.createElement('div');
  msg.className = 'meta';
  msg.textContent = `${t('panelInitFailed')}: ${String(err.message || err)}`;
  document.body.prepend(msg);
});
