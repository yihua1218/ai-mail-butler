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
