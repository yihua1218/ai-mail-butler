import React, { useState, useEffect } from 'react';
import { ConfigProvider, Layout, Menu, Typography, Button, Dropdown, Row, Col, Card, Statistic, Table, Input } from 'antd';
import { GlobalOutlined, UserOutlined, MailOutlined, MessageOutlined, LoginOutlined, LogoutOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { useAuth } from './AuthContext';
import { Chat } from './Chat';
import { About } from './About';
import axios from 'axios';

const { Header, Content, Footer } = Layout;
const { Title, Paragraph } = Typography;

const Dashboard: React.FC = () => {
  const { t } = useTranslation();
  const { user } = useAuth();
  const [personalEmails, setPersonalEmails] = useState<any[]>([]);
  const [allEmails, setAllEmails] = useState<any[]>([]);
  const [globalStats, setGlobalStats] = useState<any>(null);
  const [personalStats, setPersonalStats] = useState<any>(null);

  useEffect(() => {
    // If user is logged in, use their email, else fetch without email
    const url = user ? `/api/dashboard?email=${user.email}` : `/api/dashboard`;
    axios.get(url).then(res => {
      setGlobalStats(res.data.global_stats);
      if (res.data.type === 'admin') {
        setPersonalEmails(res.data.personal_emails);
        setPersonalStats(res.data.personal_stats);
        setAllEmails(res.data.all_emails);
      } else if (res.data.type === 'personal') {
        setPersonalEmails(res.data.personal_emails);
        setPersonalStats(res.data.personal_stats);
      }
    });
  }, [user]);

  const GlobalStatsDisplay = () => (
    globalStats ? (
      <div style={{ marginBottom: 32 }}>
        <Title level={4}>System Overview</Title>
        <Row gutter={[16, 16]}>
          <Col xs={12} sm={8} md={6}>
            <Card bordered={false} hoverable bodyStyle={{ padding: '16px' }}>
              <Statistic title={t('stats_registered_users')} value={globalStats.registered_users} prefix={<UserOutlined style={{ color: '#0071e3' }} />} />
            </Card>
          </Col>
          <Col xs={12} sm={8} md={6}>
            <Card bordered={false} hoverable bodyStyle={{ padding: '16px' }}>
              <Statistic title={t('stats_emails_received')} value={globalStats.emails_received} prefix={<MailOutlined style={{ color: '#34c759' }} />} />
            </Card>
          </Col>
          <Col xs={12} sm={8} md={6}>
            <Card bordered={false} hoverable bodyStyle={{ padding: '16px' }}>
              <Statistic title={t('stats_emails_replied')} value={globalStats.emails_replied} prefix={<MessageOutlined style={{ color: '#ff9500' }} />} />
            </Card>
          </Col>
        </Row>
      </div>
    ) : null
  );

  const PersonalStatsDisplay = () => (
    personalStats ? (
      <div style={{ marginBottom: 32 }}>
        <Title level={4}>Your Processing Status</Title>
        <Row gutter={[16, 16]}>
          <Col xs={12} sm={8} md={6}>
            <Card bordered={false} hoverable bodyStyle={{ padding: '16px' }}>
              <Statistic title="Your Received" value={personalStats.emails_received} prefix={<MailOutlined style={{ color: '#34c759' }} />} />
            </Card>
          </Col>
          <Col xs={12} sm={8} md={6}>
            <Card bordered={false} hoverable bodyStyle={{ padding: '16px' }}>
              <Statistic title="Your Replied" value={personalStats.emails_replied} prefix={<MessageOutlined style={{ color: '#ff9500' }} />} />
            </Card>
          </Col>
        </Row>
      </div>
    ) : null
  );

  const columns = [
    { title: 'Subject', dataIndex: 'subject', key: 'subject' },
    { title: 'Status', dataIndex: 'status', key: 'status' },
    { title: 'Received At', dataIndex: 'received_at', key: 'received_at' }
  ];

  if (user?.role === 'admin') {
    return (
      <div>
        <div style={{ marginBottom: 32 }}>
          <Title level={2}>{t('welcome')}, Admin ({user.email})</Title>
        </div>
        
        <GlobalStatsDisplay />
        <PersonalStatsDisplay />

        <Row gutter={[24, 24]}>
          <Col xs={24} lg={12}>
            <Card bordered={false} title="Your Emails">
              <Table dataSource={personalEmails} rowKey="id" columns={columns} pagination={{ pageSize: 5 }} />
            </Card>
          </Col>
          <Col xs={24} lg={12}>
            <Card bordered={false} title="All Processed Emails (System-wide)">
              <Table dataSource={allEmails} rowKey="id" columns={columns} pagination={{ pageSize: 5 }} />
            </Card>
          </Col>
        </Row>
      </div>
    );
  }

  if (user?.role === 'user') {
    return (
      <div>
        <div style={{ marginBottom: 32 }}>
          <Title level={2}>{t('welcome')}, {user.email}</Title>
        </div>

        <GlobalStatsDisplay />
        <PersonalStatsDisplay />

        <Card bordered={false} title="Your Emails">
          <Table dataSource={personalEmails} rowKey="id" columns={columns} />
        </Card>
      </div>
    );
  }

  // Anonymous Dashboard
  return (
    <div>
      <div style={{ marginBottom: 32 }}>
        <Title level={2}>{t('welcome')}</Title>
        <Paragraph style={{ color: '#86868b', fontSize: '16px' }}>Your intelligent email assistant is processing emails across the network.</Paragraph>
      </div>
      
      <GlobalStatsDisplay />

      <div style={{ textAlign: 'center', marginTop: 60 }}>
        <Paragraph style={{ color: '#86868b', fontSize: '16px' }}>Please login to access your personal dashboard and processing status.</Paragraph>
      </div>
    </div>
  );
};

const App: React.FC = () => {
  const { t, i18n } = useTranslation();
  const { user, requestMagicLink, verifyToken, logout, loading } = useAuth();
  const [activeMenu, setActiveMenu] = useState('1');
  const [loginEmail, setLoginEmail] = useState('test@example.com');
  const [isLinkSent, setIsLinkSent] = useState(false);

  useEffect(() => {
    const params = new URLSearchParams(window.location.search);
    const token = params.get('token');
    if (token) {
      verifyToken(token).then(() => {
        window.history.replaceState({}, document.title, window.location.pathname);
      });
    }
  }, []);

  const handleLogin = async () => {
    await requestMagicLink(loginEmail);
    setIsLinkSent(true);
    setTimeout(() => setIsLinkSent(false), 5000);
  };

  const changeLanguage = (lng: string) => {
    i18n.changeLanguage(lng);
  };

  const languageMenu = {
    items: [
      { key: 'en', label: 'English', onClick: () => changeLanguage('en') },
      { key: 'zh-TW', label: '繁體中文', onClick: () => changeLanguage('zh-TW') },
    ]
  };

  if (loading) return <div>Loading...</div>;

  return (
    <ConfigProvider
      theme={{
        token: {
          fontFamily: "-apple-system, BlinkMacSystemFont, 'SF Pro Text', 'Helvetica Neue', Arial, sans-serif",
          colorPrimary: '#0071e3', // Apple Blue
          borderRadius: 12,
          colorBgContainer: '#ffffff',
          colorBgLayout: '#f5f5f7', // Apple Light Gray background
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
          <div style={{ display: 'flex', alignItems: 'center', gap: '20px' }}>
            <Title level={4} style={{ margin: 0 }}>AI Mail Butler</Title>
            <Menu mode="horizontal" selectedKeys={[activeMenu]} onSelect={(i) => setActiveMenu(i.key)} style={{ flex: 1, minWidth: 400, border: 'none', background: 'transparent' }} items={[
              { key: '1', label: t('dashboard') },
              { key: '2', label: t('ai_chat') },
              { key: '3', label: t('settings') },
              { key: '4', label: 'About' },
            ]} />
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: 16 }}>
            {user ? (
              <Button type="text" onClick={logout} icon={<LogoutOutlined />}>Logout</Button>
            ) : (
              <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
                {isLinkSent ? (
                  <span style={{ color: '#34c759', fontSize: '14px' }}>Magic Link Sent! Check console.</span>
                ) : (
                  <>
                    <Input size="small" placeholder="Email" value={loginEmail} onChange={e => setLoginEmail(e.target.value)} />
                    <Button size="small" type="primary" onClick={handleLogin} icon={<LoginOutlined />}>Login</Button>
                  </>
                )}
              </div>
            )}
            <Dropdown menu={languageMenu} placement="bottomRight">
              <Button type="text" icon={<GlobalOutlined />}>{i18n.language === 'en' ? 'EN' : '繁中'}</Button>
            </Dropdown>
          </div>
        </Header>
        <Content style={{ padding: '40px 50px', maxWidth: '1200px', margin: '0 auto', width: '100%' }}>
          {activeMenu === '1' && <Dashboard />}
          {activeMenu === '2' && <Chat />}
          {activeMenu === '3' && <Card><p>Settings coming soon...</p></Card>}
          {activeMenu === '4' && <About />}
        </Content>
        <Footer style={{ textAlign: 'center', color: '#86868b' }}>
          AI Mail Butler ©{new Date().getFullYear()} - Released into the Public Domain
        </Footer>
      </Layout>
    </ConfigProvider>
  );
};

export default App;
