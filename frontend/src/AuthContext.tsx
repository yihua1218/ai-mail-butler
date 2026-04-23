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

export const AuthProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [user, setUser] = useState<User | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const savedEmail = localStorage.getItem('user_email');
    if (savedEmail) {
      axios.get(`/api/me?email=${savedEmail}`)
        .then(res => {
          if (res.data) setUser(res.data);
          else localStorage.removeItem('user_email');
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
    const savedEmail = localStorage.getItem('user_email');
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
      if (res.data) {
        setUser(res.data);
        localStorage.setItem('user_email', res.data.email);
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
    setUser(null);
    localStorage.removeItem('user_email');
  };

  return (
    <AuthContext.Provider value={{ user, requestMagicLink, verifyToken, refreshUser, logout, loading, api: '/api' }}>
      {children}
    </AuthContext.Provider>
  );
};

export const useAuth = () => useContext(AuthContext);
