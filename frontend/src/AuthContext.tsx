import React, { createContext, useContext, useState, useEffect } from 'react';
import axios from 'axios';

export interface User {
  id: string;
  email: string;
  is_onboarded: boolean;
  preferences?: string;
}

interface AuthContextType {
  user: User | null;
  login: (email: string) => Promise<void>;
  logout: () => void;
  loading: boolean;
}

const AuthContext = createContext<AuthContextType>({} as AuthContextType);

export const AuthProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [user, setUser] = useState<User | null>(null);
  const [loading, setLoading] = useState(true);

  const fetchUser = async (email: string) => {
    try {
      const res = await axios.get(`/api/me?email=${email}`);
      if (res.data) {
        setUser(res.data);
        localStorage.setItem('user_email', email);
      } else {
        setUser(null);
      }
    } catch (e) {
      console.error(e);
      setUser(null);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    const savedEmail = localStorage.getItem('user_email');
    if (savedEmail) {
      fetchUser(savedEmail);
    } else {
      setLoading(false);
    }
  }, []);

  const login = async (email: string) => {
    setLoading(true);
    await fetchUser(email);
  };

  const logout = () => {
    setUser(null);
    localStorage.removeItem('user_email');
  };

  return (
    <AuthContext.Provider value={{ user, login, logout, loading }}>
      {children}
    </AuthContext.Provider>
  );
};

export const useAuth = () => useContext(AuthContext);
