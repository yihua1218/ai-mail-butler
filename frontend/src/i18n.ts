import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';

// English translations
const en = {
  translation: {
    welcome: "Welcome to AI Mail Butler",
    dashboard: "Dashboard",
    settings: "Settings",
    rules: "Rules",
    about: "About",
    ai_chat: "AI Assistant",
    language: "Language",
    preferred_language: "Preferred Language",
    system_settings: "System Settings",
    processing_preferences: "Processing Preferences",
    rules_title: "Email Processing Rules",
    rules_desc: "Rules include conversational instructions from chat and rules you add manually.",
    rules_add: "Add Rule",
    rules_edit: "Edit Rule",
    rules_load_failed: "Failed to load rules.",
    rules_add_failed: "Failed to add rule.",
    rules_update_failed: "Failed to update rule.",
    rules_toggle_failed: "Failed to update rule status.",
    rules_added: "Rule added.",
    rules_updated: "Rule updated.",
    rules_source: "Source",
    rules_enabled: "Enabled",
    rules_updated_at: "Updated",
    rules_action: "Action",
    rules_rule: "Rule",
    stats_registered_users: "Registered Users",
    stats_emails_received: "Emails Received",
    stats_emails_replied: "Emails Replied",
    stats_emails_sent: "Emails Sent",
    stats_ai_replies: "AI Chat Replies",
    status_pending: "Pending",
    status_drafted: "Drafted",
    status_replied: "Replied",
    email_format: "Email Format",
    format_both: "Both (HTML + Plain)",
    format_html: "HTML Only",
    format_plain: "Plain Text Only",
    assistant_identity: "AI Assistant Identity",
    assistant_name_zh: "Assistant Chinese Name",
    assistant_name_en: "Assistant English Name",
    assistant_tone_zh: "Chinese Reply Tone",
    assistant_tone_en: "English Reply Tone",
    identity_desc: "Customize how your AI assistant identifies itself and its speaking style."
  }
};

// Traditional Chinese translations
const zhTW = {
  translation: {
    welcome: "歡迎使用 AI 郵件助理",
    dashboard: "儀表板",
    settings: "設定",
    rules: "規則",
    about: "關於",
    ai_chat: "AI 助理",
    language: "語言",
    preferred_language: "偏好語系",
    system_settings: "系統設定",
    processing_preferences: "處理偏好",
    rules_title: "電子郵件處理規則",
    rules_desc: "下列規則包含你在聊天時交代的口語指令，以及你手動新增的規則。",
    rules_add: "新增規則",
    rules_edit: "編輯規則",
    rules_load_failed: "載入規則失敗。",
    rules_add_failed: "新增規則失敗。",
    rules_update_failed: "更新規則失敗。",
    rules_toggle_failed: "更新規則狀態失敗。",
    rules_added: "已新增規則。",
    rules_updated: "已更新規則。",
    rules_source: "來源",
    rules_enabled: "啟用",
    rules_updated_at: "更新時間",
    rules_action: "操作",
    rules_rule: "規則",
    stats_registered_users: "已註冊用戶",
    stats_emails_received: "總收件數",
    stats_emails_replied: "自動回覆數",
    stats_emails_sent: "主動寄出數",
    stats_ai_replies: "AI 助理回覆次數",
    status_pending: "尚未處理",
    status_drafted: "已產生草稿",
    status_replied: "已回覆",
    email_format: "郵件格式",
    format_both: "雙格式 (HTML + 純文字)",
    format_html: "僅 HTML",
    format_plain: "僅純文字",
    assistant_identity: "AI 助理身份定義",
    assistant_name_zh: "助理中文名稱",
    assistant_name_en: "助理英文名稱",
    assistant_tone_zh: "中文回覆語氣",
    assistant_tone_en: "英文回覆語氣",
    identity_desc: "自訂 AI 助理的自我介紹名稱與說話風格。"
  }
};

i18n
  .use(initReactI18next)
  .init({
    resources: {
      en: en,
      'zh-TW': zhTW
    },
    lng: "en", // default language
    fallbackLng: "en",
    interpolation: {
      escapeValue: false
    }
  });

export default i18n;
