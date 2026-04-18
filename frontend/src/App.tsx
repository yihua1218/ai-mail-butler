import React, { useState, useEffect } from 'react';
import { ConfigProvider, Layout, Menu, Typography, Button, Dropdown, Row, Col, Card, Statistic, Table, Input } from 'antd';
import { GlobalOutlined, UserOutlined, MailOutlined, SendOutlined, MessageOutlined, LoginOutlined, LogoutOutlined } from '@ant-design/icons';
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

  useEffect(() => {
    if (user) {
      axios.get(`/api/dashboard?email=${user.email}`).then(res => {
        if (res.data.type === 'personal') {
          setPersonalEmails(res.data.emails);
        }
      });
    }
  }, [user]);

  if (user) {
    return (
      <div>
        <div style={{ marginBottom: 32 }}>
          <Title level={2}>{t('welcome')}, {user.email}</Title>
          <Paragraph style={{ color: '#86868b', fontSize: '16px' }}>Here are your forwarded emails and tasks.</Paragraph>
        </div>
        <Card bordered={false} title="Your Emails">
          <Table 
            dataSource={personalEmails} 
            rowKey="id"
            columns={[
              { title: 'Subject', dataIndex: 'subject', key: 'subject' },
              { title: 'Status', dataIndex: 'status', key: 'status' },
              { title: 'Received At', dataIndex: 'received_at', key: 'received_at' }
            ]}
          />
        </Card>
      </div>
    );
  }

  // Anonymous Dashboard
  return (
    <div>
      <div style={{ marginBottom: 32 }}>
        <Title level={2}>{t('welcome')}</Title>
        <Paragraph style={{ color: '#86868b', fontSize: '16px' }}>Your intelligent email assistant is actively processing emails.</Paragraph>
      </div>
      
      <Row gutter={[24, 24]}>
        <Col xs={24} sm={12} md={6}>
          <Card bordered={false} hoverable>
            <Statistic title={t('stats_registered_users')} value={12} prefix={<UserOutlined style={{ color: '#0071e3' }} />} />
          </Card>
        </Col>
        <Col xs={24} sm={12} md={6}>
          <Card bordered={false} hoverable>
            <Statistic title={t('stats_emails_received')} value={1248} prefix={<MailOutlined style={{ color: '#34c759' }} />} />
          </Card>
        </Col>
        <Col xs={24} sm={12} md={6}>
          <Card bordered={false} hoverable>
            <Statistic title={t('stats_emails_replied')} value={93} prefix={<MessageOutlined style={{ color: '#ff9500' }} />} />
          </Card>
        </Col>
        <Col xs={24} sm={12} md={6}>
          <Card bordered={false} hoverable>
            <Statistic title={t('stats_emails_sent')} value={45} prefix={<SendOutlined style={{ color: '#af52de' }} />} />
          </Card>
        </Col>
      </Row>
    </div>
  );
};

const App: React.FC = () => {
  const { t, i18n } = useTranslation();
  const { user, login, logout, loading } = useAuth();
  const [activeMenu, setActiveMenu] = useState('1');
  const [loginEmail, setLoginEmail] = useState('test@example.com');

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
              <div style={{ display: 'flex', gap: 8 }}>
                <Input size="small" placeholder="Email" value={loginEmail} onChange={e => setLoginEmail(e.target.value)} />
                <Button size="small" type="primary" onClick={() => login(loginEmail)} icon={<LoginOutlined />}>Login</Button>
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
