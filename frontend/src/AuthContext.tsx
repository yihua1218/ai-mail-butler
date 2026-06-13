import React, { createContext, useContext, useState, useEffect } from 'react';
import axios from 'axios';

export interface User {
  id: string;
  email: string;
  mail_send_method: string;
  is_onboarded: boolean;
  preferences: string | null;
  role: 'admin' | 'developer' | 'user';
  auto_reply: boolean;
  dry_run: boolean;
  display_name: string | null;
  email_format: string | null;
  assistant_name_zh: string | null;
  assistant_name_en: string | null;
  assistant_tone_zh: string | null;
  assistant_tone_en: string | null;
  pdf_passwords: string | null;
  timezone: string | null;
  preferred_language: string | null;
  training_data_consent: boolean;
  training_consent_updated_at: string | null;
  rule_label_mode: 'ai_first' | 'deterministic_only';
  unmatched_rule_guidance_enabled: boolean | null;
  time_format: string | null;
  date_format: string | null;
}

interface AuthContextType {
  user: User | null;
  requestMagicLink: (email: string) => Promise<void>;
  verifyToken: (token: string) => Promise<void>;
  refreshUser: () => Promise<void>;
  logout: () => void;
  loading: boolean;
  api: string;
}

const AuthContext = createContext<AuthContextType>({} as AuthContextType);

const USER_EMAIL_KEY = 'user_email';
const CSRF_COOKIE_NAME = 'csrf_token';
const CSRF_HEADER_NAME = 'x-csrf-token';

axios.defaults.withCredentials = true;

const readCookie = (name: string) => {
  if (typeof document === 'undefined') return null;
  return document.cookie
    .split(';')
    .map((part) => part.trim())
    .find((part) => part.startsWith(`${name}=`))
    ?.slice(name.length + 1) ?? null;
};

if (axios.interceptors?.request) {
  axios.interceptors.request.use((config) => {
    const method = config.method?.toUpperCase() ?? 'GET';
    if (!['GET', 'HEAD', 'OPTIONS'].includes(method)) {
      const token = readCookie(CSRF_COOKIE_NAME);
      if (token) {
        config.headers = config.headers ?? {};
        config.headers[CSRF_HEADER_NAME] = token;
      }
    }
    return config;
  });
}

export const AuthProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [user, setUser] = useState<User | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const savedEmail = localStorage.getItem(USER_EMAIL_KEY);
    if (savedEmail) {
      axios.get(`/api/me?email=${savedEmail}`)
        .then(res => {
          if (res.data) setUser(res.data);
          else {
            localStorage.removeItem(USER_EMAIL_KEY);
          }
        })
        .finally(() => setLoading(false));
    } else {
      setLoading(false);
    }
  }, []);

  const requestMagicLink = async (email: string) => {
    await axios.post('/api/auth/magic-link', { email });
  };

  const refreshUser = async () => {
    const savedEmail = localStorage.getItem(USER_EMAIL_KEY);
    if (savedEmail) {
      try {
        const res = await axios.get(`/api/me?email=${savedEmail}`);
        if (res.data) setUser(res.data);
      } catch (e) {
        console.error(e);
      }
    }
  };

  const verifyToken = async (token: string) => {
    setLoading(true);
    try {
      const res = await axios.post('/api/auth/verify', { token });
      const verifiedUser = res.data?.user ?? res.data;
      if (verifiedUser) {
        setUser(verifiedUser);
        localStorage.setItem(USER_EMAIL_KEY, verifiedUser.email);
      } else {
        throw new Error('Invalid token');
      }
    } catch (e) {
      console.error('Verification error:', e);
      throw e;
    } finally {
      setLoading(false);
    }
  };

  const logout = () => {
    axios.post('/api/auth/logout').catch(() => undefined);
    setUser(null);
    localStorage.removeItem(USER_EMAIL_KEY);
  };

  return (
    <AuthContext.Provider value={{ user, requestMagicLink, verifyToken, refreshUser, logout, loading, api: '/api' }}>
      {children}
    </AuthContext.Provider>
  );
};

export const useAuth = () => useContext(AuthContext);
