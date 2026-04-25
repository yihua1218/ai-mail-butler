import * as packagedWebLlm from './vendor/web-llm/index.js';

const DEFAULT_STATE = {
  settings: {
    mode: 'draft_assist',
    minConfidenceAutoSend: 0.9,
    blockSensitiveByDefault: true,
    localAuditEnabled: true,
    autoScanMinutes: 30,
    replyIdentityMode: 'assistant',
    userDisplayName: '',
    assistantDisplayName: 'AI Mail Butler',
    webLlmEnabled: true,
    webLlmModelId: 'Llama-3.2-1B-Instruct-q4f16_1-MLC',
    webLlmTemperature: 0.3,
    webLlmMaxTokens: 700,
  },
  rules: [],
  auditLogs: [],
  repliedThreads: {},
};

const AUTO_SCAN_ALARM = 'ai-mail-butler-auto-scan';
const MIN_AUTO_SCAN_MINUTES = 5;
const DEFAULT_AUTO_SCAN_MINUTES = 30;

const SENSITIVE_KEYWORDS = [
  'invoice', 'refund', 'legal', 'contract', 'hr', 'salary', 'medical', 'diagnosis',
  '發票', '退款', '合約', '法律', '人資', '薪資', '醫療', '病歷',
];

let webLlmEnginePromise = null;
let webLlmEngineModelId = '';
let webLlmLastStatus = {
  available: false,
  ready: false,
  modelId: DEFAULT_STATE.settings.webLlmModelId,
  progressText: 'WebLLM has not been loaded yet.',
  error: '',
};
let webLlmWarmupPromise = null;

async function getState() {
  const data = await chrome.storage.local.get(['settings', 'rules', 'auditLogs', 'repliedThreads']);
  return {
    settings: { ...DEFAULT_STATE.settings, ...(data.settings || {}) },
    rules: Array.isArray(data.rules) ? data.rules : [],
    auditLogs: Array.isArray(data.auditLogs) ? data.auditLogs : [],
    repliedThreads: data.repliedThreads && typeof data.repliedThreads === 'object' ? data.repliedThreads : {},
  };
}

async function saveState(partial) {
  await chrome.storage.local.set(partial);
}

function safeLower(input) {
  return String(input || '').toLowerCase();
}

function normalizeAutoScanMinutes(value) {
  const parsed = Number(value);
  if (!Number.isFinite(parsed) || parsed <= 0) return DEFAULT_AUTO_SCAN_MINUTES;
  return Math.max(MIN_AUTO_SCAN_MINUTES, Math.round(parsed));
}

function includesAny(text, keywords) {
  const normalized = safeLower(text);
  return keywords.some((k) => normalized.includes(safeLower(k)));
}

function isSensitiveThread(thread) {
  return includesAny(`${thread.subject || ''}\n${thread.body || ''}`, SENSITIVE_KEYWORDS);
}

function normalizeCsv(input) {
  return safeLower(input)
    .split(',')
    .map((s) => s.trim())
    .filter(Boolean);
}

function splitCsv(input) {
  return String(input || '')
    .split(',')
    .map((s) => s.trim())
    .filter(Boolean);
}

function quoteGmailTerm(term) {
  const cleaned = String(term || '').trim().replace(/\s+/g, ' ');
  if (!cleaned) return '';
  if (/^[^\s"(){}]+$/.test(cleaned)) return cleaned;
  return `"${cleaned.replaceAll('\\', '\\\\').replaceAll('"', '\\"')}"`;
}

function subjectRegexToSearchTerms(subjectRegex) {
  const raw = String(subjectRegex || '').trim();
  if (!raw) return [];

  const withoutAnchors = raw.replace(/^\^/, '').replace(/\$$/, '');
  const candidates = withoutAnchors
    .split('|')
    .map((part) =>
      part
        .trim()
        .replace(/\\([()[\]{}.?+*^$|\\])/g, '$1')
        .replace(/[()[\]{}.?+*^$|\\]/g, ' ')
        .replace(/\s+/g, ' ')
        .trim(),
    )
    .filter((part) => part.length >= 2);

  return Array.from(new Set(candidates));
}

function buildGmailSearchesForRule(rule) {
  if (!rule?.enabled) return [];

  const senderOptions = splitCsv(rule.match?.sender || '')
    .map((token) => quoteGmailTerm(token))
    .filter(Boolean)
    .map((token) => `from:${token}`);
  const subjectOptions = subjectRegexToSearchTerms(rule.match?.subject_regex || '')
    .map((term) => quoteGmailTerm(term))
    .filter(Boolean)
    .map((term) => `subject:${term}`);
  const containsParts = splitCsv(rule.match?.contains || '')
    .map((token) => quoteGmailTerm(token))
    .filter(Boolean);

  const senderVariants = senderOptions.length ? senderOptions : [''];
  const subjectVariants = subjectOptions.length ? subjectOptions : [''];
  const queries = [];

  for (const sender of senderVariants) {
    for (const subject of subjectVariants) {
      const parts = [sender, subject, ...containsParts].filter(Boolean);
      if (parts.length) queries.push(parts.join(' '));
      if (queries.length >= 24) return queries;
    }
  }

  return queries;
}

function buildGmailRuleSearches(rules) {
  return [...rules]
    .sort((a, b) => (b.priority || 0) - (a.priority || 0))
    .flatMap((rule) =>
      buildGmailSearchesForRule(rule).map((query, index) => ({
        ruleId: rule.id || '',
        ruleName: rule.name || '',
        query,
        variant: index + 1,
      })),
    );
}

function matchSender(sender, matcher) {
  if (!matcher) return true;
  const senderLower = safeLower(sender);
  return normalizeCsv(matcher).some((token) => senderLower.includes(token));
}

function matchSubjectRegex(subject, subjectRegex) {
  if (!subjectRegex) return true;
  try {
    return new RegExp(subjectRegex, 'i').test(subject || '');
  } catch {
    return false;
  }
}

function matchContains(body, contains) {
  if (!contains) return true;
  const normalizedBody = safeLower(body);
  return normalizeCsv(contains).every((token) => normalizedBody.includes(token));
}

function matchRule(rule, thread) {
  if (!rule?.enabled) return false;
  const searchableBody = `${thread.subject || ''}\n${thread.body || ''}\n${thread.sender || ''}`;
  return (
    matchSender(thread.sender, rule.match?.sender || '') &&
    matchSubjectRegex(thread.subject, rule.match?.subject_regex || '') &&
    matchContains(searchableBody, rule.match?.contains || '')
  );
}

function pickRule(rules, thread) {
  const sorted = [...rules].sort((a, b) => (b.priority || 0) - (a.priority || 0));
  return sorted.find((rule) => matchRule(rule, thread));
}

function toConfidence(thread, rule) {
  let score = 0.5;
  if (thread.subject) score += 0.1;
  if ((thread.body || '').length > 60) score += 0.1;
  if (rule?.instruction_template) score += 0.1;
  if (rule?.match?.contains) score += 0.1;
  return Math.min(0.95, score);
}

function getReplyDisplayName(settings = {}) {
  const mode = settings.replyIdentityMode === 'user' ? 'user' : 'assistant';
  if (mode === 'user') {
    return String(settings.userDisplayName || '').trim() || 'Me';
  }
  return String(settings.assistantDisplayName || '').trim() || DEFAULT_STATE.settings.assistantDisplayName;
}

function buildWebLlmMessages(thread, rule, settings) {
  const instruction = rule?.instruction_template?.trim() || 'Write a concise, helpful email reply.';
  const tone = rule?.tone_profile_id?.trim() || 'professional';
  const replyDisplayName = getReplyDisplayName(settings);
  return [
    {
      role: 'system',
      content: [
        'You are AI Mail Butler, a browser-local Gmail reply drafting assistant.',
        'Write only the email reply body. Do not mention internal rules, confidence, or implementation details.',
        'Do not invent facts, commitments, dates, prices, attachments, or approvals that are not present.',
        'If the email needs information the user has not provided, ask a brief clarifying question.',
        `Sign the email as: ${replyDisplayName}.`,
        'Do not sign with any other name.',
        `Tone profile: ${tone}.`,
      ].join('\n'),
    },
    {
      role: 'user',
      content: [
        `User instruction:\n${instruction}`,
        '',
        `Original sender:\n${thread.sender || '(unknown)'}`,
        '',
        `Original subject:\n${thread.subject || '(no subject)'}`,
        '',
        `Original email content:\n${(thread.body || '').slice(0, 6000)}`,
        '',
        'Draft the reply now.',
      ].join('\n'),
    },
  ];
}

async function loadWebLlmModule() {
  return packagedWebLlm;
}

async function ensureWebLlmEngine(settings) {
  const modelId = settings.webLlmModelId || DEFAULT_STATE.settings.webLlmModelId;
  if (webLlmEnginePromise && webLlmEngineModelId === modelId) return webLlmEnginePromise;

  webLlmEngineModelId = modelId;
  webLlmLastStatus = {
    available: false,
    ready: false,
    modelId,
    progressText: 'Loading WebLLM runtime...',
    error: '',
  };

  webLlmEnginePromise = (async () => {
    if (!navigator?.gpu) {
      throw new Error('WebGPU is unavailable in this browser profile.');
    }

    const webllm = await loadWebLlmModule();
    const create = webllm.CreateMLCEngine;
    if (typeof create !== 'function') {
      throw new Error('Packaged WebLLM module does not export CreateMLCEngine.');
    }

    const appConfig = webllm.prebuiltAppConfig
      ? { ...webllm.prebuiltAppConfig, cacheBackend: 'indexeddb' }
      : { cacheBackend: 'indexeddb' };

    const engine = await create(modelId, {
      appConfig,
      initProgressCallback: (progress) => {
        const progressText = progress?.text || (progress ? JSON.stringify(progress) : 'Loading model...');
        webLlmLastStatus = {
          available: true,
          ready: false,
          modelId,
          progressText,
          error: '',
        };
      },
    });

    webLlmLastStatus = {
      available: true,
      ready: true,
      modelId,
      progressText: 'WebLLM model is ready.',
      error: '',
    };
    return engine;
  })().catch((err) => {
    webLlmLastStatus = {
      available: false,
      ready: false,
      modelId,
      progressText: 'WebLLM unavailable; deterministic fallback will be used.',
      error: String(err?.message || err),
    };
    webLlmEnginePromise = null;
    throw err;
  });

  return webLlmEnginePromise;
}

async function warmUpWebLlmEngine() {
  const { settings } = await getState();
  const modelId = settings.webLlmModelId || DEFAULT_STATE.settings.webLlmModelId;

  if (!settings.webLlmEnabled) {
    webLlmLastStatus = {
      available: false,
      ready: false,
      modelId,
      progressText: 'WebLLM is disabled in extension settings.',
      error: '',
    };
    return webLlmLastStatus;
  }

  if (webLlmLastStatus.ready && webLlmLastStatus.modelId === modelId) return webLlmLastStatus;

  if (webLlmWarmupPromise && webLlmEngineModelId === modelId) return webLlmWarmupPromise;

  webLlmLastStatus = {
    available: true,
    ready: false,
    modelId,
    progressText: 'Starting WebLLM model load...',
    error: '',
  };

  webLlmWarmupPromise = ensureWebLlmEngine(settings)
    .then(() => webLlmLastStatus)
    .catch(() => webLlmLastStatus)
    .finally(() => {
      webLlmWarmupPromise = null;
    });

  return webLlmWarmupPromise;
}

async function generateReplyWithWebLlm(thread, rule, settings) {
  const engine = await ensureWebLlmEngine(settings);
  const response = await engine.chat.completions.create({
    messages: buildWebLlmMessages(thread, rule, settings),
    temperature: Number(settings.webLlmTemperature ?? DEFAULT_STATE.settings.webLlmTemperature),
    max_tokens: Number(settings.webLlmMaxTokens ?? DEFAULT_STATE.settings.webLlmMaxTokens),
  });

  const replyText = response?.choices?.[0]?.message?.content?.trim() || '';
  if (!replyText) {
    throw new Error('WebLLM returned an empty reply.');
  }

  return {
    replyText,
    confidence: toConfidence(thread, rule),
    model: settings.webLlmModelId || DEFAULT_STATE.settings.webLlmModelId,
    sensitivityFlags: isSensitiveThread(thread) ? ['sensitive_content'] : [],
    engine: 'webllm',
  };
}

async function generateReplyWithFallback(thread, rule, settings = {}) {
  const subject = thread.subject || 'your email';
  const shortBody = (thread.body || '').slice(0, 240);
  const tone = rule?.tone_profile_id || 'professional';
  const replyDisplayName = getReplyDisplayName(settings);

  // Local-only fallback response generator. Replace this function with WebLLM engine binding.
  const reply = [
    'Hi,',
    '',
    `Thanks for your message regarding \"${subject}\".`,
    shortBody ? `I reviewed the details: ${shortBody}` : 'I reviewed your request.',
    '',
    tone === 'concise'
      ? 'I will follow up shortly with the next step.'
      : 'I appreciate your patience. I will follow up shortly with the next step and any required details.',
    '',
    'Best regards,',
    replyDisplayName,
  ].join('\n');

  return {
    replyText: reply,
    confidence: toConfidence(thread, rule),
    model: 'deterministic-fallback',
    sensitivityFlags: isSensitiveThread(thread) ? ['sensitive_content'] : [],
    engine: 'fallback',
  };
}

async function generateReplyWithLocalEngine(thread, rule, settings) {
  if (!settings.webLlmEnabled) {
    return generateReplyWithFallback(thread, rule, settings);
  }

  try {
    return await generateReplyWithWebLlm(thread, rule, settings);
  } catch (err) {
    const fallback = await generateReplyWithFallback(thread, rule, settings);
    return {
      ...fallback,
      webLlmError: String(err?.message || err),
    };
  }
}

function decideAction({ settings, rule, confidence, hasSensitive }) {
  if (!rule) return 'ignore';

  const requested = rule.allowed_actions || settings.mode || 'draft_assist';
  if (requested !== 'guarded_autosend') return requested;

  if (hasSensitive && settings.blockSensitiveByDefault) return 'draft_assist';

  const threshold = Number(rule.min_confidence_auto_send ?? settings.minConfidenceAutoSend ?? 0.9);
  return confidence >= threshold ? 'guarded_autosend' : 'draft_assist';
}

async function appendAudit(entry) {
  const { auditLogs } = await getState();
  const next = [
    {
      id: crypto.randomUUID(),
      ts: new Date().toISOString(),
      ...entry,
    },
    ...auditLogs,
  ].slice(0, 300);
  await saveState({ auditLogs: next });
}

async function markThreadReplied(thread, action = 'draft_inserted') {
  const threadId = String(thread?.threadId || '').trim();
  if (!threadId) return null;

  const { repliedThreads } = await getState();
  const next = {
    ...repliedThreads,
    [threadId]: {
      threadId,
      sender: String(thread?.sender || '').slice(0, 200),
      subject: String(thread?.subject || '').slice(0, 200),
      action,
      ts: new Date().toISOString(),
    },
  };

  const entries = Object.entries(next)
    .sort((a, b) => String(b[1]?.ts || '').localeCompare(String(a[1]?.ts || '')))
    .slice(0, 1000);
  const trimmed = Object.fromEntries(entries);
  await saveState({ repliedThreads: trimmed });
  return trimmed[threadId];
}

async function configureAutoScanAlarm(settings) {
  if (!chrome.alarms?.create || !chrome.alarms?.clear) return;
  const periodInMinutes = normalizeAutoScanMinutes(settings?.autoScanMinutes);
  await chrome.alarms.clear(AUTO_SCAN_ALARM);
  await chrome.alarms.create(AUTO_SCAN_ALARM, {
    delayInMinutes: periodInMinutes,
    periodInMinutes,
  });
}

async function triggerAutoScanInGmailTabs() {
  if (!chrome.tabs?.query || !chrome.tabs?.sendMessage) return;
  const tabs = await chrome.tabs.query({ url: 'https://mail.google.com/*' });
  await Promise.allSettled(
    tabs
      .filter((tab) => tab.id)
      .map((tab) =>
        chrome.tabs.sendMessage(tab.id, {
          type: 'AI_MAIL_BUTLER_AUTO_SCAN',
        }),
      ),
  );
}

async function hasPermission(permissions, origins = []) {
  try {
    return await chrome.permissions.contains({ permissions, origins });
  } catch {
    return false;
  }
}

async function buildCapabilityStatus() {
  const api = {
    sidePanelApi: !!(chrome.sidePanel && typeof chrome.sidePanel.setPanelBehavior === 'function'),
    permissionsApi: !!(chrome.permissions && typeof chrome.permissions.contains === 'function'),
    webGpu: !!navigator?.gpu,
  };

  const permissions = {
    storage: await hasPermission(['storage']),
    activeTab: await hasPermission(['activeTab']),
    scripting: await hasPermission(['scripting']),
    alarms: await hasPermission(['alarms']),
    sidePanel: await hasPermission(['sidePanel']),
    gmailHost: await hasPermission([], ['https://mail.google.com/*']),
  };

  const issues = [];
  if (!api.sidePanelApi) {
    issues.push('Side Panel API is unavailable in this browser/runtime.');
  }
  if (!api.webGpu) {
    issues.push('WebGPU is unavailable; WebLLM cannot run local models in this browser profile.');
  }
  Object.entries(permissions).forEach(([name, ok]) => {
    if (!ok) issues.push(`Permission missing or not granted: ${name}`);
  });

  return {
    checkedAt: new Date().toISOString(),
    api,
    permissions,
    ok: issues.length === 0,
    issues,
  };
}

async function setupSidePanelBehavior() {
  if (!chrome.sidePanel || typeof chrome.sidePanel.setPanelBehavior !== 'function') {
    console.warn('[AI Mail Butler] Side Panel API unavailable; skip setPanelBehavior.');
    return;
  }
  try {
    await chrome.sidePanel.setPanelBehavior({ openPanelOnActionClick: true });
  } catch (err) {
    console.warn('[AI Mail Butler] setPanelBehavior failed:', err);
  }
}

chrome.runtime.onInstalled.addListener(async () => {
  const state = await getState();
  await saveState(state);
  await configureAutoScanAlarm(state.settings);
  await setupSidePanelBehavior();
});

chrome.runtime.onStartup.addListener(async () => {
  try {
    const state = await getState();
    await configureAutoScanAlarm(state.settings);
    await setupSidePanelBehavior();
  } catch (err) {
    console.warn('[AI Mail Butler] startup setup failed:', err);
  }
});

chrome.alarms?.onAlarm?.addListener((alarm) => {
  if (alarm?.name !== AUTO_SCAN_ALARM) return;
  triggerAutoScanInGmailTabs().catch((err) => {
    console.warn('[AI Mail Butler] auto scan failed:', err);
  });
});

chrome.runtime.onMessage.addListener((message, _sender, sendResponse) => {
  (async () => {
    if (message?.type === 'GET_CAPABILITY_STATUS') {
      const status = await buildCapabilityStatus();
      sendResponse({ ok: true, status });
      return;
    }

    if (message?.type === 'GET_STATE') {
      const state = await getState();
      sendResponse({ ok: true, state });
      return;
    }

    if (message?.type === 'GET_WEBLLM_STATUS') {
      sendResponse({ ok: true, status: webLlmLastStatus });
      return;
    }

    if (message?.type === 'WARM_UP_WEBLLM') {
      warmUpWebLlmEngine().catch((err) => {
        console.warn('[AI Mail Butler] WebLLM warm-up failed:', err);
      });
      sendResponse({ ok: true, status: webLlmLastStatus });
      return;
    }

    if (message?.type === 'SAVE_RULES') {
      const rules = Array.isArray(message.rules) ? message.rules : [];
      await saveState({ rules });
      sendResponse({ ok: true });
      return;
    }

    if (message?.type === 'SAVE_SETTINGS') {
      const current = await getState();
      const settings = {
        ...current.settings,
        ...(message.settings || {}),
        autoScanMinutes: normalizeAutoScanMinutes(message.settings?.autoScanMinutes ?? current.settings.autoScanMinutes),
      };
      await saveState({ settings });
      await configureAutoScanAlarm(settings);
      sendResponse({ ok: true });
      return;
    }

    if (message?.type === 'GET_GMAIL_RULE_SEARCHES') {
      const { rules } = await getState();
      sendResponse({ ok: true, searches: buildGmailRuleSearches(rules) });
      return;
    }

    if (message?.type === 'FILTER_THREADS') {
      const { rules, repliedThreads } = await getState();
      const candidates = Array.isArray(message.threads) ? message.threads : [];
      const matched = candidates
        .map((thread) => {
          const rule = pickRule(rules, thread);
          const replied = !!(thread.threadId && repliedThreads[thread.threadId]);
          return {
            threadId: thread.threadId,
            matched: !!rule,
            ruleId: rule?.id || null,
            ruleName: rule?.name || null,
            replied,
            repliedAt: replied ? repliedThreads[thread.threadId]?.ts || null : null,
          };
        })
        .filter((x) => x.matched);

      sendResponse({ ok: true, matched });
      return;
    }

    if (message?.type === 'PROCESS_THREAD') {
      const state = await getState();
      const thread = message.thread || {};
      const rule = pickRule(state.rules, thread);

      if (!rule) {
        sendResponse({ ok: true, action: 'ignore', reason: 'no_rule_matched' });
        return;
      }

      const generation = await generateReplyWithLocalEngine(thread, rule, state.settings);
      const sensitive = generation.sensitivityFlags.length > 0;
      const action = decideAction({
        settings: state.settings,
        rule,
        confidence: generation.confidence,
        hasSensitive: sensitive,
      });

      if (state.settings.localAuditEnabled) {
        await appendAudit({
          sender: thread.sender || '',
          subject: (thread.subject || '').slice(0, 120),
          ruleId: rule.id,
          action,
          confidence: generation.confidence,
          sensitivityFlags: generation.sensitivityFlags,
        });
      }

      sendResponse({
        ok: true,
        action,
        rule,
        generation,
      });
      return;
    }

    if (message?.type === 'MARK_THREAD_REPLIED') {
      const entry = await markThreadReplied(message.thread || {}, message.action || 'draft_inserted');
      sendResponse({ ok: true, entry });
      return;
    }

    sendResponse({ ok: false, error: 'unknown_message_type' });
  })().catch((err) => {
    sendResponse({ ok: false, error: String(err?.message || err) });
  });

  return true;
});

chrome.runtime.onConnect.addListener((port) => {
  if (port.name !== 'webllm-warmup') return;

  let disconnected = false;
  let intervalId = null;

  const postStatus = () => {
    if (disconnected) return;
    try {
      port.postMessage({ type: 'WEBLLM_STATUS', status: webLlmLastStatus });
    } catch {
      disconnected = true;
    }
  };

  port.onDisconnect.addListener(() => {
    disconnected = true;
    if (intervalId) clearInterval(intervalId);
  });

  port.onMessage.addListener((message) => {
    if (message?.type !== 'START_WEBLLM_WARMUP') return;

    postStatus();
    intervalId = setInterval(postStatus, 1000);

    warmUpWebLlmEngine()
      .then(postStatus)
      .catch((err) => {
        console.warn('[AI Mail Butler] WebLLM port warm-up failed:', err);
        postStatus();
      })
      .finally(() => {
        if (intervalId) clearInterval(intervalId);
        setTimeout(() => {
          postStatus();
          try {
            port.disconnect();
          } catch {
            // Port may already be closed by the side panel.
          }
        }, 250);
      });
  });
});
