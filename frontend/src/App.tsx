import React, { Suspense, lazy, useEffect, useRef, useState } from 'react';
import { Alert, Button, ConfigProvider, Dropdown, Input, Layout, Menu, Spin, Typography, message } from 'antd';
import { GlobalOutlined, LoginOutlined, LogoutOutlined } from '@ant-design/icons';
import axios from 'axios';
import { useTranslation } from 'react-i18next';
import { useNavigate, useLocation } from 'react-router-dom';
import { useAuth } from './AuthContext';

const DashboardPage = lazy(() => import('./pages/DashboardPage'));
const SettingsPage = lazy(() => import('./pages/SettingsPage'));
const RulesManagerPage = lazy(() => import('./pages/RulesManagerPage'));
const FinanceAnalysisPage = lazy(() => import('./pages/FinanceAnalysisPage'));
const GdprDeletePage = lazy(() => import('./pages/GdprDeletePage'));
const PrivacyPage = lazy(() => import('./pages/PrivacyPage').then(m => ({ default: m.default as React.ComponentType })));
const ChatPage = lazy(() => import('./Chat').then((m) => ({ default: m.Chat })));
const AboutPage = lazy(() => import('./About').then((m) => ({ default: m.About })));
const HowItWorksPage = lazy(() => import('./pages/HowItWorksPage'));
const WebLlmLocalPage = lazy(() => import('./pages/WebLlmLocalPage'));

const { Header, Content, Footer } = Layout;
const { Title } = Typography;

// Map URL path → menu key and vice-versa
const PATH_TO_KEY: Record<string, string> = {
  '/': '1',
  '/dashboard': '1',
  '/chat': '2',
  '/settings': '3',
  '/about': '4',
  '/rules': '5',
  '/finance': '6',
  '/privacy': '7',
  '/how-it-works': '8',
  '/webllm-local': '9',
  '/gdpr-delete': '0',
  '/login': '1', // Login also maps to dashboard view
};
const KEY_TO_PATH: Record<string, string> = {
  '1': '/dashboard',
  '2': '/chat',
  '3': '/settings',
  '4': '/about',
  '5': '/rules',
  '6': '/finance',
  '7': '/privacy',
  '8': '/how-it-works',
  '9': '/webllm-local',
};

const App: React.FC = () => {
  const { t, i18n } = useTranslation();
  const { user, requestMagicLink, verifyToken, logout, loading } = useAuth();
  const navigate = useNavigate();
  const location = useLocation();
  const [loginEmail, setLoginEmail] = useState('');
  const [isLinkSent, setIsLinkSent] = useState(false);
  const [readonlyInfo, setReadonlyInfo] = useState<{ enabled: boolean; overlayDir?: string; readonlyBase?: string }>({ enabled: false });

  // Derive active menu key from current URL path
  const activeMenu = PATH_TO_KEY[location.pathname] ?? '1';

  const verificationStarted = useRef(false);

  // Handle magic link token in URL (e.g. /login?token=...)
  useEffect(() => {
    const params = new URLSearchParams(location.search);
    const token = params.get('token');
    
    if (location.pathname === '/login' && token && !verificationStarted.current && !user) {
      verificationStarted.current = true;
      console.log('Token found in URL, verifying...');
      verifyToken(token).then(() => {
        console.log('Token verified, navigating to dashboard');
        navigate('/dashboard', { replace: true });
        message.success('Successfully logged in!');
      }).catch(err => {
        console.error('Token verification failed:', err);
        message.error('Invalid or expired login link.');
        navigate('/dashboard', { replace: true });
        verificationStarted.current = false;
      });
    }
  }, [location.pathname, location.search, navigate, user, verifyToken]);

  // Update document.title based on current page
  useEffect(() => {
    const titles: Record<string, string> = {
      '1': t('dashboard'),
      '2': t('ai_chat'),
      '3': t('settings'),
      '4': t('about'),
      '5': t('rules'),
      '6': t('finance'),
      '7': t('privacy.title'),
      '8': t('how_it_works'),
      '9': t('webllm_local'),
    };
    document.title = `${titles[activeMenu] ?? 'AI Mail Butler'} | AI Mail Butler`;
  }, [activeMenu, t]);

  useEffect(() => {
    axios.get('/api/about')
      .then((res) => {
        setReadonlyInfo({
          enabled: !!res.data?.readonly_mode_enabled,
          overlayDir: res.data?.overlay_dir || undefined,
          readonlyBase: res.data?.readonly_base || undefined,
        });
      })
      .catch(() => {
        setReadonlyInfo({ enabled: false });
      });
  }, []);

  const handleMenuSelect = (key: string) => {
    navigate(KEY_TO_PATH[key] ?? '/dashboard');
  };

  const handleLogin = async () => {
    if (!loginEmail.trim()) return;
    await requestMagicLink(loginEmail);
    setIsLinkSent(true);
    setTimeout(() => setIsLinkSent(false), 5000);
  };

  const changeLanguage = (lng: string) => {
    i18n.changeLanguage(lng);
    // Only persist to localStorage for guests; logged-in users use DB preference
    if (!user) {
      localStorage.setItem('i18n_lang', lng);
    }
  };

  // When user logs in, switch UI language to the preference stored in the DB
  useEffect(() => {
    if (user?.preferred_language) {
      i18n.changeLanguage(user.preferred_language);
    }
  }, [user?.preferred_language]);

  const languageMenu = {
    items: [
      { key: 'en', label: 'English', onClick: () => changeLanguage('en') },
      { key: 'zh-TW', label: '繁體中文', onClick: () => changeLanguage('zh-TW') },
    ]
  };

  if (loading) return <div style={{ display: 'flex', justifyContent: 'center', alignItems: 'center', height: '100vh' }}>Authenticating...</div>;

  const pageFallback = (
    <div style={{ display: 'flex', justifyContent: 'center', alignItems: 'center', minHeight: 240 }}>
      <Spin size="large" />
    </div>
  );

  const renderPage = () => {
    switch (activeMenu) {
      case '1':
        return <DashboardPage />;
      case '2':
        return <ChatPage />;
      case '3':
        return <SettingsPage />;
      case '4':
        return <AboutPage />;
      case '5':
        return <RulesManagerPage />;
      case '6':
        return <FinanceAnalysisPage />;
      case '7':
        return <PrivacyPage />;
      case '8':
        return <HowItWorksPage />;
      case '9':
        return <WebLlmLocalPage />;
      default:
        return <DashboardPage />;
    }
  };

  if (location.pathname === '/gdpr-delete') {
    return (
      <ConfigProvider
        theme={{
          token: {
            fontFamily: "-apple-system, BlinkMacSystemFont, 'SF Pro Text', 'Helvetica Neue', Arial, sans-serif",
            colorPrimary: '#0071e3',
            borderRadius: 12,
          },
        }}
      >
        <Layout style={{ minHeight: '100vh' }}>
          <Content style={{ padding: '40px 50px', maxWidth: '1000px', margin: '0 auto', width: '100%' }}>
            <Suspense fallback={pageFallback}>
              <GdprDeletePage />
            </Suspense>
          </Content>
        </Layout>
      </ConfigProvider>
    );
  }

  return (
    <ConfigProvider
      theme={{
        token: {
          fontFamily: "-apple-system, BlinkMacSystemFont, 'SF Pro Text', 'Helvetica Neue', Arial, sans-serif",
          colorPrimary: '#0071e3',
          borderRadius: 12,
          colorBgContainer: '#ffffff',
          colorBgLayout: '#f5f5f7',
        },
        components: {
          Layout: {
            headerBg: 'rgba(255, 255, 255, 0.8)',
            headerColor: '#1d1d1f',
          },
          Card: {
            boxShadowTertiary: '0 4px 12px rgba(0,0,0,0.05)',
          }
        }
      }}
    >
      <Layout style={{ minHeight: '100vh' }}>
        <Header style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', backdropFilter: 'saturate(180%) blur(20px)', position: 'sticky', top: 0, zIndex: 1, borderBottom: '1px solid rgba(0,0,0,0.1)' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '20px', flex: 1, minWidth: 0 }}>
            <Title level={4} style={{ margin: 0, cursor: 'pointer', flexShrink: 0 }} onClick={() => navigate('/')}>AI Mail Butler</Title>
            <Menu
              mode="horizontal"
              selectedKeys={[activeMenu]}
              onSelect={(i) => handleMenuSelect(i.key)}
              style={{ flex: 1, minWidth: 0, border: 'none', background: 'transparent' }}
              items={[
                { key: '1', label: t('dashboard') },
                { key: '2', label: t('ai_chat') },
                { key: '3', label: t('settings') },
                { key: '5', label: t('rules') },
                { key: '6', label: t('finance') },
                { key: '7', label: t('privacy.title') },
                { key: '8', label: t('how_it_works') },
                { key: '9', label: t('webllm_local') },
                { key: '4', label: t('about') },
              ]}
            />
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: 16 }}>
            {user ? (
              <div style={{ display: 'flex', gap: 12, alignItems: 'center' }}>
                <span style={{ color: '#1d1d1f', fontWeight: 500 }}>{user.display_name || user.email}</span>
                <Button size="small" onClick={logout} icon={<LogoutOutlined />}>{t('logout')}</Button>
              </div>
            ) : (
              <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
                {isLinkSent ? (
                  <span style={{ color: '#34c759', fontSize: '14px' }}>{t('magic_link_sent')}</span>
                ) : (
                  <>
                    <Input
                      size="small"
                      placeholder="your@email.com"
                      value={loginEmail}
                      onChange={e => setLoginEmail(e.target.value)}
                      onPressEnter={handleLogin}
                      style={{ width: 200 }}
                    />
                    <Button size="small" type="primary" onClick={handleLogin} icon={<LoginOutlined />}>{t('login')}</Button>
                  </>
                )}
              </div>
            )}
            <Dropdown menu={languageMenu} placement="bottomRight">
              <Button type="text" icon={<GlobalOutlined />}>{i18n.language === 'en' ? 'EN' : '繁中'}</Button>
            </Dropdown>
          </div>
        </Header>
        <Content
          style={{
            padding: '32px clamp(16px, 3vw, 52px)',
            width: '100%',
          }}
        >
          {readonlyInfo.enabled && (
            <Alert
              type="warning"
              showIcon
              style={{ marginBottom: 16 }}
              message={i18n.language === 'zh-TW' ? '唯讀 Overlay 模式啟用中' : 'Read-only Overlay Mode Enabled'}
              description={i18n.language === 'zh-TW'
                ? `系統會阻擋所有寫入 API。Overlay: ${readonlyInfo.overlayDir || '-'}；Base: ${readonlyInfo.readonlyBase || '-'}`
                : `All write APIs are blocked. Overlay: ${readonlyInfo.overlayDir || '-'}; Base: ${readonlyInfo.readonlyBase || '-'}.`}
            />
          )}
          <Suspense fallback={pageFallback}>
            {renderPage()}
          </Suspense>
        </Content>
        <Footer style={{ textAlign: 'center', color: '#86868b' }}>
          AI Mail Butler ©{new Date().getFullYear()} - Released into the Public Domain
        </Footer>
      </Layout>
    </ConfigProvider>
  );
};

export default App;
