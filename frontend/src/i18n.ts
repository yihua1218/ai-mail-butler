import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';

// English translations
const en = {
  translation: {
    welcome: "Welcome to AI Mail Butler",
    dashboard: "Dashboard",
    settings: "Settings",
    ai_chat: "AI Assistant",
    language: "Language",
    stats_registered_users: "Registered Users",
    stats_emails_received: "Emails Received",
    stats_emails_replied: "Emails Replied",
    stats_emails_sent: "Emails Sent",
    stats_ai_replies: "AI Chat Replies",
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
    ai_chat: "AI 助理",
    language: "語言",
    stats_registered_users: "已註冊用戶",
    stats_emails_received: "總收件數",
    stats_emails_replied: "自動回覆數",
    stats_emails_sent: "主動寄出數",
    stats_ai_replies: "AI 助理回覆次數",
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
