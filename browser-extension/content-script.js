(() => {
const PANEL_ID = 'ai-mail-butler-local-panel';
const HIGHLIGHT_ATTR = 'data-ai-mail-butler-match';
const REPLIED_ATTR = 'data-ai-mail-butler-replied';
const LANG_KEY = 'extension_ui_lang';
const CONTENT_SCRIPT_VERSION = '2026-04-26-1';

const I18N = {
  en: {
    panelTitle: 'AI Mail Butler (Local)',
    scan: 'Search again once',
    process: 'Generate reply for open thread',
    insertDraft: 'Insert draft',
    sendNow: 'Send now',
    ready: 'Ready.',
    scanning: 'Scanning visible Gmail threads...',
    searchingByRules: 'Searching Gmail by saved rules...',
    searchableRuleProgress: 'Searching rule {{current}}/{{total}}: {{rule}}',
    processing: 'Processing with local engine...',
    matchedThreads: 'Matched {{count}} visible thread(s).',
    matchedThreadsBySearch: 'Found {{count}} matching thread(s) across Gmail rule searches.',
    matchedThreadsByAutoSearch: 'Auto scan found {{count}} matching thread(s).',
    noRules: 'No saved searchable rules. Add a rule with sender, subject, or body keywords first.',
    noOpenThread: 'No open Gmail thread detected. Open a conversation and try again.',
    noRuleMatched: 'No rule matched this thread.',
    action: 'Action',
    rule: 'Rule',
    confidence: 'Confidence',
    draft: 'Draft',
    unnamed: 'unnamed',
    guardedConfirm: 'This rule allows guarded auto-send. Send this message now?',
    noDraft: 'No generated draft to insert yet.',
    noCompose: 'Cannot find the Gmail compose box. Open compose and try again.',
    noReplyCompose: 'Cannot find the Gmail reply box. Open the message and try again.',
    draftInserted: 'Draft inserted.',
    draftInsertedAuto: 'Draft inserted. This thread matched an auto-send eligible rule.',
    sendButtonNotFound: 'Cannot find the Gmail Send button. Open the compose window and try again.',
    sendTriggered: 'Send action triggered.',
    scanFailedTitle: 'Scan failed.',
    processFailedTitle: 'Reply generation failed.',
    extensionUnavailable: 'The extension connection is unavailable in this tab.',
    extensionUnavailableNext: 'This usually happens after the extension was reloaded. Reload the Gmail tab and try again.',
    runtimeMissing: 'Messaging runtime is unavailable in this page.',
    runtimeMissingNext: 'Reload the Gmail tab, then reopen the AI panel.',
    backgroundUnavailable: 'The extension background service is not responding.',
    backgroundUnavailableNext: 'Open the extension page, check for errors, then reload the Gmail tab.',
    unknownError: 'Unknown error',
    nextStep: 'Next step',
  },
  'zh-TW': {
    panelTitle: 'AI Mail Butler（本地端）',
    scan: '重新搜尋一次',
    process: '為目前開啟的信件產生回覆',
    insertDraft: '插入草稿',
    sendNow: '立即寄出',
    ready: '就緒。',
    scanning: '正在掃描目前可見的 Gmail 信件...',
    searchingByRules: '正在依已儲存規則搜尋 Gmail...',
    searchableRuleProgress: '正在搜尋規則 {{current}}/{{total}}：{{rule}}',
    processing: '正在用本地模型處理中...',
    matchedThreads: '已命中 {{count}} 封目前可見的信件。',
    matchedThreadsBySearch: '已在 Gmail 規則搜尋中找到 {{count}} 封命中信件。',
    matchedThreadsByAutoSearch: '自動掃描找到 {{count}} 封命中信件。',
    noRules: '目前沒有可搜尋的已儲存規則。請先新增含寄件者、主旨或內容關鍵字的規則。',
    noOpenThread: '目前沒有偵測到已開啟的 Gmail 對話。請先打開一封信再重試。',
    noRuleMatched: '這封信目前沒有命中任何規則。',
    action: '動作',
    rule: '規則',
    confidence: '信心分數',
    draft: '草稿',
    unnamed: '未命名',
    guardedConfirm: '此規則允許條件式自動寄送。要現在直接送出嗎？',
    noDraft: '目前還沒有可插入的草稿內容。',
    noCompose: '找不到 Gmail 撰寫視窗。請先打開撰寫視窗後再重試。',
    noReplyCompose: '找不到 Gmail 回覆視窗。請先打開信件後再重試。',
    draftInserted: '已插入草稿。',
    draftInsertedAuto: '已插入草稿。這封信命中了可自動寄送的規則。',
    sendButtonNotFound: '找不到 Gmail 的寄送按鈕。請先確認撰寫視窗已開啟後再重試。',
    sendTriggered: '已觸發寄送動作。',
    scanFailedTitle: '掃描失敗。',
    processFailedTitle: '產生回覆失敗。',
    extensionUnavailable: '此分頁目前無法連到 Extension。',
    extensionUnavailableNext: '這通常發生在你剛重新載入 Extension 之後。請重新整理 Gmail 分頁，再試一次。',
    runtimeMissing: '此頁面的訊息傳遞 Runtime 目前不可用。',
    runtimeMissingNext: '請重新整理 Gmail 分頁，然後再打開 AI 面板。',
    backgroundUnavailable: 'Extension 背景服務目前沒有回應。',
    backgroundUnavailableNext: '請打開 Extension 管理頁檢查錯誤，然後重新整理 Gmail 分頁。',
    unknownError: '未知錯誤',
    nextStep: '下一步',
  },
};

let currentLang = (navigator.language || '').toLowerCase().startsWith('zh') ? 'zh-TW' : 'en';
let lastReplyText = '';
let lastAction = 'ignore';
let lastProcessedThread = null;

function isVisible(node) {
  if (!(node instanceof HTMLElement)) return false;
  const style = window.getComputedStyle(node);
  const rect = node.getBoundingClientRect();
  return style.display !== 'none' && style.visibility !== 'hidden' && rect.width > 0 && rect.height > 0;
}

function pickFirstVisible(selectors, root = document) {
  for (const selector of selectors) {
    const nodes = Array.from(root.querySelectorAll(selector));
    const found = nodes.find(isVisible);
    if (found) return found;
  }
  return null;
}

function pickAllVisible(selectors, root = document) {
  const seen = new Set();
  const results = [];
  for (const selector of selectors) {
    const nodes = Array.from(root.querySelectorAll(selector)).filter(isVisible);
    for (const node of nodes) {
      if (seen.has(node)) continue;
      seen.add(node);
      results.push(node);
    }
  }
  return results;
}

function t(key, vars = {}) {
  const template = I18N[currentLang]?.[key] ?? I18N.en[key] ?? key;
  return Object.entries(vars).reduce((acc, [k, v]) => acc.replaceAll(`{{${k}}}`, String(v)), template);
}

function stableHash(input) {
  let hash = 0;
  const text = String(input || '');
  for (let i = 0; i < text.length; i += 1) {
    hash = (hash * 31 + text.charCodeAt(i)) >>> 0;
  }
  return hash.toString(36);
}

function compactText(input) {
  return String(input || '').replace(/\s+/g, ' ').trim();
}

function delay(ms) {
  return new Promise((resolve) => {
    window.setTimeout(resolve, ms);
  });
}

async function loadLangPreference() {
  try {
    if (chrome?.storage?.local) {
      const data = await chrome.storage.local.get([LANG_KEY]);
      currentLang = data[LANG_KEY] === 'zh-TW' ? 'zh-TW' : data[LANG_KEY] === 'en' ? 'en' : currentLang;
    }
  } catch {
    // Ignore and keep browser-language fallback.
  }
}

function getOpenThread() {
  const main = document.querySelector('div[role="main"]') || document;
  const subjectNode = pickFirstVisible(['h2.hP', 'h2[data-thread-perm-id]', 'div[role="main"] h2'], main);
  const subject = subjectNode?.textContent?.trim() || '';

  const messages = pickAllVisible(['div.adn.ads', 'div[role="listitem"][data-message-id]', 'div[data-message-id]'], main);
  const openMessage = messages[messages.length - 1] || main;
  const senderNode = pickFirstVisible(['span.gD[email]', 'span[email]', '[data-hovercard-id]'], openMessage);
  const sender =
    senderNode?.getAttribute?.('email') ||
    senderNode?.getAttribute?.('data-hovercard-id') ||
    senderNode?.textContent?.trim() ||
    '';

  const bodyRoot = messages.length ? main : openMessage;
  const bodyNodes = pickAllVisible(['div.a3s', 'div.a3s.aiL', 'div.ii.gt div[dir="auto"]', 'div.ii.gt div[dir="ltr"]'], bodyRoot);
  const body = bodyNodes
    .map((n) => compactText(n.textContent || ''))
    .filter(Boolean)
    .join('\n')
    .trim();

  const hash = window.location.hash || '';
  const threadMatch =
    hash.match(/(?:^|[/?&])th=([^&]+)/) ||
    hash.match(/#(?:inbox|sent|all|search|category\/[^/]+)\/([^/?]+)/) ||
    hash.match(/\/([^/?]+)$/);
  const threadId = threadMatch?.[1] || stableHash(`${sender}\n${subject}\n${body.slice(0, 500)}`);

  return { threadId, sender, subject, body };
}

function getVisibleInboxThreads() {
  const rows = Array.from(
    document.querySelectorAll(
      [
        'tr.zA',
        'div[role="main"] div[role="link"][data-legacy-thread-id]',
        'div[role="main"] div[role="link"][data-thread-id]',
        'div[role="main"] div[role="listitem"][data-legacy-thread-id]',
        'div[role="main"] div[role="listitem"][data-thread-id]',
      ].join(','),
    ),
  ).filter(isVisible);

  return rows.map((row, index) => {
    const threadIdRaw =
      row.getAttribute('data-legacy-thread-id') ||
      row.getAttribute('data-thread-id') ||
      row.querySelector('[data-legacy-thread-id]')?.getAttribute('data-legacy-thread-id') ||
      row.querySelector('[data-thread-id]')?.getAttribute('data-thread-id') ||
      '';
    const sender = compactText(
      row.querySelector('span[email]')?.getAttribute('email') ||
        row.querySelector('.yW span[email]')?.getAttribute('email') ||
        row.querySelector('.yW span')?.textContent ||
        row.querySelector('[email]')?.getAttribute('email') ||
        '',
    );
    const subject = compactText(
      row.querySelector('.bog')?.textContent ||
        row.querySelector('[data-thread-perm-id]')?.textContent ||
        row.querySelector('[role="link"] span[id]')?.textContent ||
        '',
    );
    const snippet = compactText(row.querySelector('.y2')?.textContent || row.getAttribute('aria-label') || row.textContent || '');
    const threadId = threadIdRaw || `visible-${index}-${stableHash(`${sender}\n${subject}\n${snippet}`)}`;
    return { threadId, sender, subject, body: snippet, row };
  });
}

function isThreadListView() {
  return getVisibleInboxThreads().length > 0;
}

function getThreadRowLink(row) {
  if (!(row instanceof HTMLElement)) return null;
  if (row.matches('tr.zA')) {
    return pickFirstVisible(['td[role="link"]', '.bog', '.yW span', '.y2'], row) || row;
  }
  return row;
}

function openThreadRow(row) {
  const target = getThreadRowLink(row);
  if (!(target instanceof HTMLElement)) return false;
  target.click();
  return true;
}

async function waitForOpenThread(threadId, timeoutMs = 8000) {
  const started = Date.now();
  while (Date.now() - started < timeoutMs) {
    await delay(250);
    const thread = getOpenThread();
    if (thread.subject || thread.body) {
      if (!threadId || thread.threadId === threadId || window.location.hash.includes(threadId)) return thread;
    }
  }
  return getOpenThread();
}

function findLatestUnrepliedMatch(rows, matched) {
  const matchedById = new Map(matched.map((item) => [item.threadId, item]));
  return rows.find(({ threadId }) => {
    const match = threadId ? matchedById.get(threadId) : null;
    return match && !match.replied;
  }) || null;
}

function ensurePanel() {
  let panel = document.getElementById(PANEL_ID);
  if (panel && panel.getAttribute('data-version') === CONTENT_SCRIPT_VERSION) return panel;
  if (panel) panel.remove();

  panel = document.createElement('div');
  panel.id = PANEL_ID;
  panel.style.position = 'fixed';
  panel.style.right = '18px';
  panel.style.bottom = '18px';
  panel.style.width = '360px';
  panel.style.maxHeight = '65vh';
  panel.style.overflow = 'auto';
  panel.style.zIndex = '2147483646';
  panel.style.background = '#fff';
  panel.style.border = '1px solid #d9d9d9';
  panel.style.borderRadius = '12px';
  panel.style.boxShadow = '0 12px 40px rgba(0,0,0,0.18)';
  panel.style.padding = '12px';
  panel.style.fontFamily = "-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif";
  panel.style.fontSize = '13px';
  panel.setAttribute('data-version', CONTENT_SCRIPT_VERSION);

  panel.innerHTML = `
    <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:8px;">
      <strong id="ai-mail-butler-title">${t('panelTitle')}</strong>
      <button id="ai-mail-butler-close" style="border:0;background:transparent;cursor:pointer;font-size:16px;">×</button>
    </div>
    <div style="display:grid;gap:8px;">
      <button id="ai-mail-butler-scan" style="padding:8px;border-radius:8px;border:1px solid #1677ff;background:#f0f7ff;cursor:pointer;">${t('scan')}</button>
      <button id="ai-mail-butler-process" style="padding:8px;border-radius:8px;border:1px solid #13a10e;background:#f1fff0;cursor:pointer;">${t('process')}</button>
      <div id="ai-mail-butler-output" style="white-space:pre-wrap;background:#fafafa;border:1px solid #eee;border-radius:8px;padding:8px;min-height:88px;">${t('ready')}</div>
      <div style="display:flex;gap:8px;">
        <button id="ai-mail-butler-insert" style="flex:1;padding:8px;border-radius:8px;border:1px solid #ccc;cursor:pointer;">${t('insertDraft')}</button>
        <button id="ai-mail-butler-send" style="flex:1;padding:8px;border-radius:8px;border:1px solid #ffb3b3;background:#fff5f5;cursor:pointer;">${t('sendNow')}</button>
      </div>
    </div>
  `;

  document.body.appendChild(panel);
  panel.querySelector('#ai-mail-butler-close')?.addEventListener('click', () => panel?.remove());
  return panel;
}

function setOutput(text) {
  const panel = ensurePanel();
  const output = panel.querySelector('#ai-mail-butler-output');
  if (output) output.textContent = text;
}

function setOutputIfVisible(text) {
  const output = document.getElementById('ai-mail-butler-output');
  if (output) output.textContent = text;
}

function formatUserError(title, reason, nextStep) {
  return `${title}\n\n${reason}\n\n${t('nextStep')}: ${nextStep}`;
}

function classifyMessagingError(err) {
  const raw = String(err?.message || err || '');
  if (!chrome?.runtime) {
    return formatUserError(t('scanFailedTitle'), t('runtimeMissing'), t('runtimeMissingNext'));
  }
  if (raw.includes('Extension context invalidated') || raw.includes('message port closed') || raw.includes('sendMessage')) {
    return formatUserError(t('scanFailedTitle'), t('extensionUnavailable'), t('extensionUnavailableNext'));
  }
  return formatUserError(t('scanFailedTitle'), raw || t('unknownError'), t('backgroundUnavailableNext'));
}

function classifyProcessError(err) {
  const raw = String(err?.message || err || '');
  if (!chrome?.runtime) {
    return formatUserError(t('processFailedTitle'), t('runtimeMissing'), t('runtimeMissingNext'));
  }
  if (raw.includes('Extension context invalidated') || raw.includes('message port closed') || raw.includes('sendMessage')) {
    return formatUserError(t('processFailedTitle'), t('extensionUnavailable'), t('extensionUnavailableNext'));
  }
  return formatUserError(t('processFailedTitle'), raw || t('unknownError'), t('backgroundUnavailableNext'));
}

async function sendRuntimeMessage(message) {
  if (!chrome?.runtime?.id || !chrome?.runtime?.sendMessage) {
    throw new Error('RUNTIME_UNAVAILABLE');
  }
  return chrome.runtime.sendMessage(message);
}

async function markThreadReplied(action) {
  if (!lastProcessedThread?.threadId) return;
  try {
    await sendRuntimeMessage({
      type: 'MARK_THREAD_REPLIED',
      action,
      thread: lastProcessedThread,
    });
  } catch (err) {
    console.warn('[AI Mail Butler] Failed to mark thread as replied:', err);
  }
}

function clearThreadHighlights(rows = getVisibleInboxThreads()) {
  rows.forEach(({ row }) => {
    row.removeAttribute(HIGHLIGHT_ATTR);
    row.removeAttribute(REPLIED_ATTR);
    row.style.outline = '';
    row.style.background = '';
  });
}

async function filterAndHighlightVisibleThreads() {
  const rows = getVisibleInboxThreads();
  const payload = rows.map(({ row, ...rest }) => rest);
  const res = await sendRuntimeMessage({ type: 'FILTER_THREADS', threads: payload });
  if (!res?.ok) {
    throw new Error(res?.error || t('unknownError'));
  }

  clearThreadHighlights(rows);
  const matchedById = new Map(res.matched.map((m) => [m.threadId, m]));
  rows.forEach(({ threadId, row }) => {
    const match = threadId ? matchedById.get(threadId) : null;
    if (!match) return;

    row.setAttribute(HIGHLIGHT_ATTR, '1');
    if (match.replied) {
      row.setAttribute(REPLIED_ATTR, '1');
      row.style.outline = '2px solid #1677ff';
      row.style.background = '#e6f4ff';
    } else {
      row.style.outline = '2px solid #52c41a';
      row.style.background = '#f6ffed';
    }
  });

  return res.matched;
}

function findGmailSearchInput() {
  return pickFirstVisible(
    [
      'form[role="search"] input[name="q"]',
      'input[aria-label="Search mail"]',
      'input[placeholder="Search mail"]',
      'input[name="q"]',
    ],
    document,
  );
}

function setNativeInputValue(input, value) {
  const descriptor = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(input), 'value');
  if (descriptor?.set) {
    descriptor.set.call(input, value);
  } else {
    input.value = value;
  }
  input.dispatchEvent(new Event('input', { bubbles: true }));
  input.dispatchEvent(new Event('change', { bubbles: true }));
}

async function waitForSearchResults(previousSignature) {
  const startedAt = Date.now();
  while (Date.now() - startedAt < 8000) {
    await delay(250);
    const rows = getVisibleInboxThreads();
    const signature = rows.map((item) => `${item.threadId}:${item.subject}`).join('|');
    if (rows.length && signature !== previousSignature) return rows;

    const mainText = compactText((document.querySelector('div[role="main"]') || document.body).textContent || '');
    if (/No messages matched your search|No results found|找不到|沒有郵件/.test(mainText)) return rows;
  }

  return getVisibleInboxThreads();
}

async function runGmailSearch(query) {
  const input = findGmailSearchInput();
  if (!(input instanceof HTMLInputElement)) {
    throw new Error('Cannot find Gmail search box.');
  }

  const previousRows = getVisibleInboxThreads();
  const previousSignature = previousRows.map((item) => `${item.threadId}:${item.subject}`).join('|');
  setNativeInputValue(input, query);

  const form = input.closest('form');
  if (form instanceof HTMLFormElement) {
    form.requestSubmit();
  } else {
    input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', code: 'Enter', bubbles: true }));
    input.dispatchEvent(new KeyboardEvent('keyup', { key: 'Enter', code: 'Enter', bubbles: true }));
  }

  await waitForSearchResults(previousSignature);
}

async function scanVisibleThreads({ quiet = false, auto = false, openLatestUnreplied = true } = {}) {
  const writeOutput = quiet ? setOutputIfVisible : setOutput;
  writeOutput(t('searchingByRules'));
  const searchRes = await sendRuntimeMessage({ type: 'GET_GMAIL_RULE_SEARCHES' });
  if (!searchRes?.ok) {
    writeOutput(formatUserError(t('scanFailedTitle'), searchRes?.error || t('unknownError'), t('backgroundUnavailableNext')));
    return;
  }

  const searches = Array.isArray(searchRes.searches) ? searchRes.searches : [];
  if (!searches.length) {
    writeOutput(t('noRules'));
    return;
  }

  const matchedByThread = new Map();
  let lastMatchedQuery = '';
  let latestUnrepliedCandidate = null;
  for (let index = 0; index < searches.length; index += 1) {
    const search = searches[index];
    writeOutput(t('searchableRuleProgress', { current: index + 1, total: searches.length, rule: search.ruleName || search.ruleId || search.query }));
    await runGmailSearch(search.query);
    const matched = await filterAndHighlightVisibleThreads();
    if (matched.length) lastMatchedQuery = search.query;
    if (!latestUnrepliedCandidate) {
      const candidate = findLatestUnrepliedMatch(getVisibleInboxThreads(), matched);
      if (candidate) {
        latestUnrepliedCandidate = {
          query: search.query,
          threadId: candidate.threadId,
        };
      }
    }
    matched.forEach((item) => {
      if (item.threadId) matchedByThread.set(item.threadId, item);
    });
  }

  if (lastMatchedQuery && !getVisibleInboxThreads().some(({ row }) => row.getAttribute(HIGHLIGHT_ATTR) === '1')) {
    await runGmailSearch(lastMatchedQuery);
    await filterAndHighlightVisibleThreads();
  }

  writeOutput(t(auto ? 'matchedThreadsByAutoSearch' : 'matchedThreadsBySearch', { count: matchedByThread.size }));

  if (openLatestUnreplied && latestUnrepliedCandidate) {
    await openLatestUnrepliedMatchedThread(latestUnrepliedCandidate, writeOutput);
  }
}

async function openLatestUnrepliedMatchedThread(candidate, writeOutput = setOutput) {
  await runGmailSearch(candidate.query);
  const matched = await filterAndHighlightVisibleThreads();
  const rows = getVisibleInboxThreads();
  const matchedById = new Map(matched.map((item) => [item.threadId, item]));
  const target =
    rows.find(({ threadId }) => {
      const match = threadId ? matchedById.get(threadId) : null;
      return threadId === candidate.threadId && match && !match.replied;
    }) ||
    findLatestUnrepliedMatch(rows, matched);

  if (!target?.row || !openThreadRow(target.row)) return;

  await waitForOpenThread(target.threadId);
  await processOpenThread({ autoInsertDraft: true, writeOutput });
}

async function processOpenThread({ autoInsertDraft = false, writeOutput = setOutput } = {}) {
  const thread = getOpenThread();
  if (!thread.subject && !thread.body) {
    writeOutput(t('noOpenThread'));
    return;
  }

  writeOutput(t('processing'));
  const res = await sendRuntimeMessage({ type: 'PROCESS_THREAD', thread });
  if (!res?.ok) {
    writeOutput(formatUserError(t('processFailedTitle'), res?.error || t('unknownError'), t('backgroundUnavailableNext')));
    return;
  }

  if (res.action === 'ignore') {
    writeOutput(t('noRuleMatched'));
    return;
  }

  lastReplyText = res.generation?.replyText || '';
  lastAction = res.action;
  lastProcessedThread = thread;

  writeOutput(
    [
      `${t('action')}: ${res.action}`,
      `${t('rule')}: ${res.rule?.name || res.rule?.id || t('unnamed')}`,
      `${t('confidence')}: ${Number(res.generation?.confidence || 0).toFixed(2)}`,
      res.generation?.model ? `Model: ${res.generation.model}` : '',
      res.generation?.webLlmError ? `WebLLM: ${res.generation.webLlmError}` : '',
      '',
      `${t('draft')}:`,
      lastReplyText,
    ].filter((line) => line !== '').join('\n'),
  );

  let insertedDraft = false;
  if (autoInsertDraft && res.action !== 'ignore') {
    insertedDraft = await insertDraft({ writeOutput, requireReply: true });
  }

  if (res.action === 'guarded_autosend') {
    const inserted = autoInsertDraft ? insertedDraft : await insertDraft({ writeOutput, requireReply: true });
    const ok = inserted && window.confirm(t('guardedConfirm'));
    if (ok) {
      await clickSend();
    }
  }
}

function findComposeBody() {
  const editors = Array.from(
    document.querySelectorAll(
      'div[aria-label="Message Body"][contenteditable="true"], div[contenteditable="true"][role="textbox"], div[contenteditable="true"][g_editable="true"]',
    ),
  ).filter(isVisible);
  return editors[editors.length - 1] || null;
}

function openReplyComposer({ allowNewCompose = false } = {}) {
  if (findComposeBody()) return true;

  const messages = pickAllVisible(['div.adn.ads', 'div[role="listitem"][data-message-id]', 'div[data-message-id]']);
  const openMessage = messages[messages.length - 1] || pickFirstVisible(['div.adn.ads', 'div[role="listitem"]', 'div[data-message-id]']);
  const replyButton = pickFirstVisible(
    [
      'div[role="button"][data-tooltip^="Reply"]',
      'div[role="button"][aria-label^="Reply"]',
      'div[role="button"][aria-label^="回覆"]',
      'div[role="button"][data-tooltip^="回覆"]',
      'span[role="button"][aria-label^="Reply"]',
      'span[role="button"][aria-label^="回覆"]',
    ],
    openMessage || document,
  );

  if (replyButton instanceof HTMLElement) {
    replyButton.click();
    return true;
  }

  const inlineReplyLink = pickFirstVisible(
    [
      'div[role="link"][aria-label^="Reply"]',
      'div[role="link"][aria-label^="回覆"]',
      'span[role="link"][aria-label^="Reply"]',
      'span[role="link"][aria-label^="回覆"]',
    ],
    openMessage || document,
  );
  if (inlineReplyLink instanceof HTMLElement) {
    inlineReplyLink.click();
    return true;
  }

  const composeButton = allowNewCompose ? document.querySelector('div[gh="cm"]') : null;
  if (composeButton instanceof HTMLElement) {
    composeButton.click();
    return true;
  }

  return false;
}

async function waitForComposeBody(timeoutMs = 3000) {
  const existing = findComposeBody();
  if (existing) return existing;

  const started = Date.now();
  while (Date.now() - started < timeoutMs) {
    await new Promise((resolve) => setTimeout(resolve, 100));
    const editor = findComposeBody();
    if (editor) return editor;
  }
  return null;
}

function setComposeBodyText(editor, text) {
  if (!(editor instanceof HTMLElement)) return;
  editor.focus();
  try {
    document.execCommand('selectAll', false);
    document.execCommand('insertText', false, text);
  } catch {
    editor.textContent = text;
    editor.dispatchEvent(new InputEvent('input', { bubbles: true, data: text, inputType: 'insertText' }));
  }
}

async function insertDraft({ writeOutput = setOutput, requireReply = false } = {}) {
  if (!lastReplyText) {
    writeOutput(t('noDraft'));
    return false;
  }

  openReplyComposer({ allowNewCompose: !requireReply });
  const editor = await waitForComposeBody();
  if (!editor) {
    writeOutput(t(requireReply ? 'noReplyCompose' : 'noCompose'));
    return false;
  }

  setComposeBodyText(editor, lastReplyText);
  await markThreadReplied('draft_inserted');
  writeOutput(lastAction === 'guarded_autosend' ? t('draftInsertedAuto') : t('draftInserted'));
  return true;
}

async function clickSend() {
  const sendBtn = document.querySelector('div[role="button"][data-tooltip^="Send"]');
  if (!(sendBtn instanceof HTMLElement)) {
    setOutput(t('sendButtonNotFound'));
    return;
  }
  sendBtn.click();
  await markThreadReplied('sent');
  setOutput(t('sendTriggered'));
}

function bindPanelActions() {
  const panel = ensurePanel();
  if (panel.getAttribute('data-bound') === '1') return;

  panel.querySelector('#ai-mail-butler-scan')?.addEventListener('click', () => {
    scanVisibleThreads().catch((err) => setOutput(classifyMessagingError(err)));
  });
  panel.querySelector('#ai-mail-butler-process')?.addEventListener('click', () => {
    processOpenThread().catch((err) => setOutput(classifyProcessError(err)));
  });
  panel.querySelector('#ai-mail-butler-insert')?.addEventListener('click', () => {
    insertDraft().catch((err) => setOutput(classifyProcessError(err)));
  });
  panel.querySelector('#ai-mail-butler-send')?.addEventListener('click', () => {
    clickSend().catch((err) => setOutput(classifyProcessError(err)));
  });
  panel.setAttribute('data-bound', '1');

  if (isThreadListView() && panel.getAttribute('data-initial-rule-search') !== '1') {
    panel.setAttribute('data-initial-rule-search', '1');
    window.setTimeout(() => {
      scanVisibleThreads().catch((err) => setOutput(classifyMessagingError(err)));
    }, 250);
  }
}

chrome.runtime?.onMessage?.addListener((message, _sender, sendResponse) => {
  if (message?.type !== 'AI_MAIL_BUTLER_AUTO_SCAN') return false;

  scanVisibleThreads({ quiet: true, auto: true })
    .then(() => sendResponse({ ok: true }))
    .catch((err) => sendResponse({ ok: false, error: String(err?.message || err) }));
  return true;
});

function installFloatingButton() {
  const existing = document.getElementById('ai-mail-butler-launcher');
  if (existing?.getAttribute('data-version') === CONTENT_SCRIPT_VERSION) return;
  if (existing) existing.remove();

  const btn = document.createElement('button');
  btn.id = 'ai-mail-butler-launcher';
  btn.setAttribute('data-version', CONTENT_SCRIPT_VERSION);
  btn.textContent = 'AI';
  btn.style.position = 'fixed';
  btn.style.right = '20px';
  btn.style.bottom = '20px';
  btn.style.zIndex = '2147483647';
  btn.style.width = '48px';
  btn.style.height = '48px';
  btn.style.borderRadius = '50%';
  btn.style.border = 'none';
  btn.style.cursor = 'pointer';
  btn.style.fontWeight = '700';
  btn.style.background = 'linear-gradient(135deg,#1677ff,#13a10e)';
  btn.style.color = '#fff';
  btn.style.boxShadow = '0 8px 24px rgba(0,0,0,0.22)';
  btn.addEventListener('click', () => {
    ensurePanel();
    bindPanelActions();
  });
  document.body.appendChild(btn);
}

loadLangPreference().finally(() => {
  installFloatingButton();
});
})();
