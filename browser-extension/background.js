const DEFAULT_STATE = {
  settings: {
    mode: 'draft_assist',
    minConfidenceAutoSend: 0.9,
    blockSensitiveByDefault: true,
    localAuditEnabled: true,
  },
  rules: [],
  auditLogs: [],
};

const SENSITIVE_KEYWORDS = [
  'invoice', 'refund', 'legal', 'contract', 'hr', 'salary', 'medical', 'diagnosis',
  '發票', '退款', '合約', '法律', '人資', '薪資', '醫療', '病歷',
];

async function getState() {
  const data = await chrome.storage.local.get(['settings', 'rules', 'auditLogs']);
  return {
    settings: { ...DEFAULT_STATE.settings, ...(data.settings || {}) },
    rules: Array.isArray(data.rules) ? data.rules : [],
    auditLogs: Array.isArray(data.auditLogs) ? data.auditLogs : [],
  };
}

async function saveState(partial) {
  await chrome.storage.local.set(partial);
}

function safeLower(input) {
  return String(input || '').toLowerCase();
}

function includesAny(text, keywords) {
  const normalized = safeLower(text);
  return keywords.some((k) => normalized.includes(safeLower(k)));
}

function isSensitiveThread(thread) {
  return includesAny(`${thread.subject || ''}\n${thread.body || ''}`, SENSITIVE_KEYWORDS);
}

function matchSender(sender, matcher) {
  if (!matcher) return true;
  const senderLower = safeLower(sender);
  return safeLower(matcher)
    .split(',')
    .map((s) => s.trim())
    .filter(Boolean)
    .some((token) => senderLower.includes(token));
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
  return safeLower(contains)
    .split(',')
    .map((s) => s.trim())
    .filter(Boolean)
    .every((token) => normalizedBody.includes(token));
}

function matchRule(rule, thread) {
  if (!rule?.enabled) return false;
  return (
    matchSender(thread.sender, rule.match?.sender || '') &&
    matchSubjectRegex(thread.subject, rule.match?.subject_regex || '') &&
    matchContains(thread.body, rule.match?.contains || '')
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

async function generateReplyWithLocalEngine(thread, rule) {
  const subject = thread.subject || 'your email';
  const shortBody = (thread.body || '').slice(0, 240);
  const tone = rule?.tone_profile_id || 'professional';

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
    'AI Mail Butler',
  ].join('\n');

  return {
    replyText: reply,
    confidence: toConfidence(thread, rule),
    model: 'local-browser-engine',
    sensitivityFlags: isSensitiveThread(thread) ? ['sensitive_content'] : [],
  };
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
  await setupSidePanelBehavior();
});

chrome.runtime.onStartup.addListener(() => {
  setupSidePanelBehavior().catch((err) => {
    console.warn('[AI Mail Butler] startup side panel setup failed:', err);
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

    if (message?.type === 'SAVE_RULES') {
      const rules = Array.isArray(message.rules) ? message.rules : [];
      await saveState({ rules });
      sendResponse({ ok: true });
      return;
    }

    if (message?.type === 'SAVE_SETTINGS') {
      const current = await getState();
      await saveState({ settings: { ...current.settings, ...(message.settings || {}) } });
      sendResponse({ ok: true });
      return;
    }

    if (message?.type === 'FILTER_THREADS') {
      const { rules } = await getState();
      const candidates = Array.isArray(message.threads) ? message.threads : [];
      const matched = candidates
        .map((thread) => {
          const rule = pickRule(rules, thread);
          return {
            threadId: thread.threadId,
            matched: !!rule,
            ruleId: rule?.id || null,
            ruleName: rule?.name || null,
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

      const generation = await generateReplyWithLocalEngine(thread, rule);
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

    sendResponse({ ok: false, error: 'unknown_message_type' });
  })().catch((err) => {
    sendResponse({ ok: false, error: String(err?.message || err) });
  });

  return true;
});
