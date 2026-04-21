import React, { useState, useEffect, useRef } from 'react';
import { 
  Layout, Menu, Typography, Card, Table, Row, Col, Statistic, Button, 
  Input, Form, Switch, message, Dropdown, ConfigProvider, Alert, Radio, Tag, Badge, Space, Modal, Select
} from 'antd';
import { GlobalOutlined, UserOutlined, MailOutlined, MessageOutlined, LoginOutlined, LogoutOutlined, SettingOutlined, RobotOutlined, WarningOutlined, PlusOutlined, EditOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { useNavigate, useLocation } from 'react-router-dom';
import { useAuth } from './AuthContext';
import { Chat, GUEST_NAME_KEY } from './Chat';
import { About } from './About';
import axios from 'axios';

const { Header, Content, Footer } = Layout;
const { Title, Paragraph } = Typography;

// Map URL path → menu key and vice-versa
const PATH_TO_KEY: Record<string, string> = {
  '/': '1',
  '/dashboard': '1',
  '/chat': '2',
  '/settings': '3',
  '/about': '4',
  '/rules': '5',
  '/login': '1', // Login also maps to dashboard view
};
const KEY_TO_PATH: Record<string, string> = {
  '1': '/dashboard',
  '2': '/chat',
  '3': '/settings',
  '4': '/about',
  '5': '/rules',
};

const Dashboard: React.FC = () => {
  const { t, i18n } = useTranslation();
  const { user } = useAuth();
  const [personalEmails, setPersonalEmails] = useState<any[]>([]);
  const [allEmails, setAllEmails] = useState<any[]>([]);
  const [globalStats, setGlobalStats] = useState<any>(null);
  const [personalStats, setPersonalStats] = useState<any>(null);
  const [mailErrors, setMailErrors] = useState<any[]>([]);

  useEffect(() => {
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
    if (user?.role === 'admin') {
      axios.get(`/api/admin/errors?email=${user.email}`).then(res => {
        setMailErrors(res.data.errors || []);
      }).catch(() => {});
    }
  }, [user]);

  const GlobalStatsDisplay = () => (
    globalStats ? (
      <div style={{ marginBottom: 32 }}>
        <Title level={4}>System Overview</Title>
        <Row gutter={[16, 16]}>
          <Col xs={12} sm={8} md={6}>
            <Card bordered={false} hoverable styles={{ body: { padding: '16px' } }}>
              <Statistic title={t('stats_registered_users')} value={globalStats.registered_users} prefix={<UserOutlined style={{ color: '#0071e3' }} />} />
            </Card>
          </Col>
          <Col xs={12} sm={8} md={6}>
            <Card bordered={false} hoverable styles={{ body: { padding: '16px' } }}>
              <Statistic title={t('stats_emails_received')} value={globalStats.emails_received} prefix={<MailOutlined style={{ color: '#34c759' }} />} />
            </Card>
          </Col>
          <Col xs={12} sm={8} md={6}>
            <Card bordered={false} hoverable styles={{ body: { padding: '16px' } }}>
              <Statistic title={t('stats_emails_replied')} value={globalStats.emails_replied} prefix={<MessageOutlined style={{ color: '#ff9500' }} />} />
            </Card>
          </Col>
          <Col xs={12} sm={8} md={6}>
            <Card bordered={false} hoverable styles={{ body: { padding: '16px' } }}>
              <Statistic title={t('stats_ai_replies')} value={globalStats.ai_replies ?? 0} prefix={<RobotOutlined style={{ color: '#af52de' }} />} />
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
            <Card bordered={false} hoverable styles={{ body: { padding: '16px' } }}>
              <Statistic title="Your Received" value={personalStats.emails_received} prefix={<MailOutlined style={{ color: '#34c759' }} />} />
            </Card>
          </Col>
          <Col xs={12} sm={8} md={6}>
            <Card bordered={false} hoverable styles={{ body: { padding: '16px' } }}>
              <Statistic title="Your Replied" value={personalStats.emails_replied} prefix={<MessageOutlined style={{ color: '#ff9500' }} />} />
            </Card>
          </Col>
        </Row>
      </div>
    ) : null
  );

  const columns = [
    { title: 'Subject', dataIndex: 'subject', key: 'subject' },
    {
      title: 'Status',
      dataIndex: 'status',
      key: 'status',
      render: (value: string) => {
        const statusColor: Record<string, string> = {
          pending: 'gold',
          drafted: 'blue',
          replied: 'green',
        };
        const label = t(`status_${value}`, { defaultValue: value });
        return <Tag color={statusColor[value] || 'default'}>{label}</Tag>;
      }
    },
    {
      title: 'Received At',
      dataIndex: 'received_at',
      key: 'received_at',
      render: (value: string) => {
        if (!value) return '-';
        const timezone = user?.timezone || 'UTC';
        const iso = value.includes('T') ? value : `${value.replace(' ', 'T')}Z`;
        const date = new Date(iso);
        if (Number.isNaN(date.getTime())) return value;
        return new Intl.DateTimeFormat(i18n.language === 'zh-TW' ? 'zh-TW' : 'en-US', {
          timeZone: timezone,
          year: 'numeric',
          month: '2-digit',
          day: '2-digit',
          hour: '2-digit',
          minute: '2-digit',
          second: '2-digit',
          hour12: false,
        }).format(date);
      }
    }
  ];

  const errorTypeColor: Record<string, string> = {
    smtp_connect: 'red',
    smtp_send: 'orange',
    ai_error: 'purple',
  };

  const errorColumns = [
    {
      title: 'Type',
      dataIndex: 'error_type',
      key: 'error_type',
      width: 120,
      render: (v: string) => <Tag color={errorTypeColor[v] || 'default'}>{v}</Tag>,
    },
    { title: 'Message', dataIndex: 'message', key: 'message', ellipsis: true },
    { title: 'Context', dataIndex: 'context', key: 'context', width: 180, ellipsis: true },
    { title: 'Time', dataIndex: 'occurred_at', key: 'occurred_at', width: 170 },
  ];

  if (user?.role === 'admin') {
    return (
      <div>
        <div style={{ marginBottom: 32 }}>
          <Title level={2}>{t('welcome')}, Admin ({user.display_name || user.email})</Title>
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
          <Col xs={24}>
            <Card
              bordered={false}
              title={
                <span>
                  <WarningOutlined style={{ color: '#ff4d4f', marginRight: 8 }} />
                  Mail Server Error Log
                  {mailErrors.length > 0 && <Badge count={mailErrors.length} style={{ marginLeft: 8, backgroundColor: '#ff4d4f' }} />}
                </span>
              }
            >
              {mailErrors.length === 0
                ? <Alert message="No errors recorded." type="success" showIcon />
                : <Table dataSource={mailErrors} rowKey="id" columns={errorColumns} pagination={{ pageSize: 10 }} size="small" />}
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
          <Title level={2}>{t('welcome')}, {user.display_name || user.email}</Title>
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
  const guestName = localStorage.getItem(GUEST_NAME_KEY);
  return (
    <div>
      <div style={{ marginBottom: 32 }}>
        <Title level={2}>{t('welcome')}{guestName ? `, ${guestName}` : ''}</Title>
        <Paragraph style={{ color: '#86868b', fontSize: '16px' }}>Your intelligent email assistant is processing emails across the network.</Paragraph>
      </div>
      <GlobalStatsDisplay />
      <div style={{ textAlign: 'center', marginTop: 60 }}>
        <Paragraph style={{ color: '#86868b', fontSize: '16px' }}>Please login to access your personal dashboard and processing status.</Paragraph>
      </div>
    </div>
  );
};

const Settings: React.FC = () => {
  const { t } = useTranslation();
  const { user, refreshUser } = useAuth();
  const [loading, setLoading] = useState(false);
  const [form] = Form.useForm();

  useEffect(() => {
    if (user) {
      let pdfPasswords: string[] = [];
      if (user.pdf_passwords) {
        try {
          const parsed = JSON.parse(user.pdf_passwords);
          if (Array.isArray(parsed)) {
            pdfPasswords = parsed;
          }
        } catch {
          pdfPasswords = [];
        }
      }

      form.setFieldsValue({
        display_name: user.display_name,
        auto_reply: user.auto_reply,
        dry_run: user.dry_run,
        email_format: user.email_format,
        timezone: user.timezone || 'UTC',
        assistant_name_zh: user.assistant_name_zh,
        assistant_name_en: user.assistant_name_en,
        assistant_tone_zh: user.assistant_tone_zh,
        assistant_tone_en: user.assistant_tone_en,
        pdf_passwords: pdfPasswords,
      });
    } else {
      // Guest settings from localStorage
      form.setFieldsValue({
        display_name: localStorage.getItem(GUEST_NAME_KEY) || '',
      });
    }
  }, [user, form]);

  const onFinish = async (values: any) => {
    if (!user) {
      // Save guest settings locally
      localStorage.setItem(GUEST_NAME_KEY, values.display_name || '');
      message.success('Guest settings saved locally!');
      return;
    }

    setLoading(true);
    try {
      await axios.post('/api/settings', { email: user.email, ...values });
      message.success('Settings saved successfully!');
      refreshUser();
    } catch {
      message.error('Failed to save settings.');
    } finally {
      setLoading(false);
    }
  };

  return (
    <Card title="System Settings" bordered={false} style={{ maxWidth: 650, margin: '0 auto' }}>
      {!user && (
        <Alert 
          message="Guest Mode" 
          description="You are currently not logged in. Settings like your nickname will be stored only in this browser." 
          type="info" 
          showIcon 
          style={{ marginBottom: 20 }}
        />
      )}
      <Form form={form} layout="vertical" onFinish={onFinish}>
        <Title level={5} style={{ marginBottom: 16 }}>Personal Info</Title>
        <Form.Item name="display_name" label="Your Display Name (顯示名稱)" tooltip="How the AI assistant and system should call you.">
          <Input placeholder="e.g. Yihua, Master, etc." />
        </Form.Item>
        
        {user && (
          <>
            <Title level={5} style={{ margin: '24px 0 16px' }}>{t('assistant_identity')}</Title>
            <Paragraph style={{ color: '#86868b', fontSize: '12px', marginBottom: 16 }}>{t('identity_desc')}</Paragraph>
            <Row gutter={16}>
              <Col span={12}>
                <Form.Item name="assistant_name_zh" label={t('assistant_name_zh')}>
                  <Input placeholder="e.g. 小管家" />
                </Form.Item>
              </Col>
              <Col span={12}>
                <Form.Item name="assistant_name_en" label={t('assistant_name_en')}>
                  <Input placeholder="e.g. Butler" />
                </Form.Item>
              </Col>
            </Row>
            <Row gutter={16}>
              <Col span={12}>
                <Form.Item name="assistant_tone_zh" label={t('assistant_tone_zh')} tooltip="設定中文回覆時的語氣，例如：專業、親切、簡短、幽默。">
                  <Input placeholder="e.g. 專業且快速" />
                </Form.Item>
              </Col>
              <Col span={12}>
                <Form.Item name="assistant_tone_en" label={t('assistant_tone_en')} tooltip="Set the tone for English replies, e.g. professional, friendly, concise, witty.">
                  <Input placeholder="e.g. witty and brief" />
                </Form.Item>
              </Col>
            </Row>

            <Title level={5} style={{ margin: '24px 0 16px' }}>Processing Preferences</Title>
            <Form.Item name="dry_run" label="Dry Run Mode (試運行模式)" valuePropName="checked" tooltip="When enabled, AI replies are drafted and sent to your own email for review. When disabled, they are sent directly to the original sender.">
              <Switch />
            </Form.Item>
            <Form.Item name="auto_reply" label="Auto Reply (自動回覆)" valuePropName="checked" tooltip="Automatically send the AI-generated reply. If Dry Run is off, this will send it to the external sender.">
              <Switch />
            </Form.Item>
            <Form.Item name="email_format" label={t('email_format')} tooltip="Choose how magic links and notifications are sent to you.">
              <Radio.Group>
                <Radio value="both">{t('format_both')}</Radio>
                <Radio value="html">{t('format_html')}</Radio>
                <Radio value="plain">{t('format_plain')}</Radio>
              </Radio.Group>
            </Form.Item>

            <Form.Item
              name="timezone"
              label="Timezone"
              tooltip="Used to display local times in Dashboard."
              rules={[{ required: true, message: 'Please select a timezone' }]}
            >
              <Select
                showSearch
                options={[
                  { value: 'UTC', label: 'UTC' },
                  { value: 'Asia/Taipei', label: 'Asia/Taipei (UTC+8)' },
                  { value: 'Asia/Tokyo', label: 'Asia/Tokyo (UTC+9)' },
                  { value: 'Asia/Shanghai', label: 'Asia/Shanghai (UTC+8)' },
                  { value: 'Asia/Singapore', label: 'Asia/Singapore (UTC+8)' },
                  { value: 'America/Los_Angeles', label: 'America/Los_Angeles' },
                  { value: 'America/New_York', label: 'America/New_York' },
                  { value: 'Europe/London', label: 'Europe/London' },
                  { value: 'Europe/Berlin', label: 'Europe/Berlin' },
                ]}
              />
            </Form.Item>

            <Title level={5} style={{ margin: '24px 0 16px' }}>PDF Passwords</Title>
            <Paragraph style={{ color: '#86868b', fontSize: '12px', marginBottom: 16 }}>
              If some PDF attachments are encrypted, add the passwords here so the system can decode and convert them into Markdown.
            </Paragraph>
            <Form.List name="pdf_passwords">
              {(fields, { add, remove }) => (
                <>
                  {fields.map((field) => (
                    <Space key={field.key} style={{ display: 'flex', marginBottom: 8 }} align="baseline">
                      <Form.Item
                        {...field}
                        style={{ marginBottom: 0, minWidth: 340 }}
                        rules={[{ required: true, message: 'Password cannot be empty' }]}
                      >
                        <Input.Password placeholder="PDF password" />
                      </Form.Item>
                      <Button onClick={() => remove(field.name)} danger>
                        Remove
                      </Button>
                    </Space>
                  ))}
                  <Form.Item>
                    <Button type="dashed" onClick={() => add()} icon={<PlusOutlined />}>
                      Add PDF Password
                    </Button>
                  </Form.Item>
                </>
              )}
            </Form.List>
          </>
        )}
        <Form.Item style={{ marginTop: 32 }}>
          <Button type="primary" htmlType="submit" loading={loading} icon={<SettingOutlined />} block>
            Save Settings
          </Button>
        </Form.Item>
      </Form>
    </Card>
  );
};

type EmailRule = {
  id: number;
  rule_text: string;
  source: string;
  is_enabled: boolean;
  updated_at?: string;
};

const RulesManager: React.FC = () => {
  const { user } = useAuth();
  const [rules, setRules] = useState<EmailRule[]>([]);
  const [newRule, setNewRule] = useState('');
  const [saving, setSaving] = useState(false);
  const [editingRule, setEditingRule] = useState<EmailRule | null>(null);
  const [editText, setEditText] = useState('');

  const loadRules = async () => {
    if (!user) return;
    try {
      const res = await axios.get(`/api/rules?email=${encodeURIComponent(user.email)}`);
      setRules(res.data.rules || []);
    } catch {
      message.error('Failed to load rules.');
    }
  };

  useEffect(() => {
    loadRules();
  }, [user?.email]);

  const addRule = async () => {
    if (!user || !newRule.trim()) return;
    setSaving(true);
    try {
      await axios.post('/api/rules/create', { email: user.email, rule_text: newRule.trim() });
      setNewRule('');
      message.success('Rule added.');
      loadRules();
    } catch {
      message.error('Failed to add rule.');
    } finally {
      setSaving(false);
    }
  };

  const toggleRule = async (rule: EmailRule, enabled: boolean) => {
    if (!user) return;
    try {
      await axios.post('/api/rules/toggle', { email: user.email, id: rule.id, is_enabled: enabled });
      loadRules();
    } catch {
      message.error('Failed to update rule status.');
    }
  };

  const saveEdit = async () => {
    if (!user || !editingRule || !editText.trim()) return;
    setSaving(true);
    try {
      await axios.post('/api/rules/update', {
        email: user.email,
        id: editingRule.id,
        rule_text: editText.trim(),
      });
      setEditingRule(null);
      setEditText('');
      message.success('Rule updated.');
      loadRules();
    } catch {
      message.error('Failed to update rule.');
    } finally {
      setSaving(false);
    }
  };

  const columns = [
    {
      title: 'Rule',
      dataIndex: 'rule_text',
      key: 'rule_text',
      render: (v: string) => <span>{v}</span>,
    },
    {
      title: 'Source',
      dataIndex: 'source',
      key: 'source',
      width: 120,
      render: (v: string) => <Tag color={v === 'chat' ? 'cyan' : 'default'}>{v}</Tag>,
    },
    {
      title: 'Enabled',
      dataIndex: 'is_enabled',
      key: 'is_enabled',
      width: 120,
      render: (_: boolean, record: EmailRule) => (
        <Switch checked={record.is_enabled} onChange={(checked) => toggleRule(record, checked)} />
      ),
    },
    {
      title: 'Updated',
      dataIndex: 'updated_at',
      key: 'updated_at',
      width: 200,
    },
    {
      title: 'Action',
      key: 'action',
      width: 120,
      render: (_: unknown, record: EmailRule) => (
        <Button
          icon={<EditOutlined />}
          onClick={() => {
            setEditingRule(record);
            setEditText(record.rule_text);
          }}
        >
          Edit
        </Button>
      ),
    },
  ];

  if (!user) {
    return (
      <Card bordered={false}>
        <Alert
          type="info"
          showIcon
          message="Please login first"
          description="Rule management is available for logged-in users only."
        />
      </Card>
    );
  }

  return (
    <Card bordered={false} title="Email Processing Rules">
      <Paragraph style={{ color: '#666' }}>
        Rules below include conversational instructions you told the assistant in chat, plus manual rules you add here.
      </Paragraph>
      <Space.Compact style={{ width: '100%', marginBottom: 16 }}>
        <Input
          placeholder="Example: 如果是信用卡帳單，先摘要重點再提醒繳費期限"
          value={newRule}
          onChange={(e) => setNewRule(e.target.value)}
          onPressEnter={addRule}
        />
        <Button type="primary" icon={<PlusOutlined />} onClick={addRule} loading={saving}>
          Add Rule
        </Button>
      </Space.Compact>

      <Table rowKey="id" dataSource={rules} columns={columns} pagination={{ pageSize: 8 }} />

      <Modal
        title="Edit Rule"
        open={!!editingRule}
        onCancel={() => {
          setEditingRule(null);
          setEditText('');
        }}
        onOk={saveEdit}
        confirmLoading={saving}
      >
        <Input.TextArea rows={5} value={editText} onChange={(e) => setEditText(e.target.value)} />
      </Modal>
    </Card>
  );
};

const App: React.FC = () => {
  const { t, i18n } = useTranslation();
  const { user, requestMagicLink, verifyToken, logout, loading } = useAuth();
  const navigate = useNavigate();
  const location = useLocation();
  const [loginEmail, setLoginEmail] = useState('');
  const [isLinkSent, setIsLinkSent] = useState(false);

  // Derive active menu key from current URL path
  const activeMenu = PATH_TO_KEY[location.pathname] ?? '1';

  const verificationStarted = useRef(false);

  // Handle magic link token in URL (e.g. /login?token=...)
  useEffect(() => {
    const params = new URLSearchParams(location.search);
    const token = params.get('token');
    
    if (token && !verificationStarted.current && !user) {
      verificationStarted.current = true;
      console.log('Token found in URL, verifying...', token);
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
  }, [location.search, user]);

  // Update document.title based on current page
  useEffect(() => {
    const titles: Record<string, string> = {
      '1': t('dashboard'),
      '2': t('ai_chat'),
      '3': t('settings'),
      '4': 'About',
      '5': 'Rules',
    };
    document.title = `${titles[activeMenu] ?? 'AI Mail Butler'} | AI Mail Butler`;
  }, [activeMenu, t]);

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
  };

  const languageMenu = {
    items: [
      { key: 'en', label: 'English', onClick: () => changeLanguage('en') },
      { key: 'zh-TW', label: '繁體中文', onClick: () => changeLanguage('zh-TW') },
    ]
  };

  if (loading) return <div style={{ display: 'flex', justifyContent: 'center', alignItems: 'center', height: '100vh' }}>Authenticating...</div>;

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
          <div style={{ display: 'flex', alignItems: 'center', gap: '20px' }}>
            <Title level={4} style={{ margin: 0, cursor: 'pointer' }} onClick={() => navigate('/')}>AI Mail Butler</Title>
            <Menu
              mode="horizontal"
              selectedKeys={[activeMenu]}
              onSelect={(i) => handleMenuSelect(i.key)}
              style={{ flex: 1, minWidth: 400, border: 'none', background: 'transparent' }}
              items={[
                { key: '1', label: t('dashboard') },
                { key: '2', label: t('ai_chat') },
                { key: '3', label: t('settings') },
                { key: '4', label: 'About' },
                { key: '5', label: 'Rules' },
              ]}
            />
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: 16 }}>
            {user ? (
              <div style={{ display: 'flex', gap: 12, alignItems: 'center' }}>
                <span style={{ color: '#1d1d1f', fontWeight: 500 }}>{user.display_name || user.email}</span>
                <Button size="small" onClick={logout} icon={<LogoutOutlined />}>Logout</Button>
              </div>
            ) : (
              <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
                {isLinkSent ? (
                  <span style={{ color: '#34c759', fontSize: '14px' }}>Magic Link Sent! Check console.</span>
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
          {activeMenu === '3' && <Settings />}
          {activeMenu === '4' && <About />}
          {activeMenu === '5' && <RulesManager />}
        </Content>
        <Footer style={{ textAlign: 'center', color: '#86868b' }}>
          AI Mail Butler ©{new Date().getFullYear()} - Released into the Public Domain
        </Footer>
      </Layout>
    </ConfigProvider>
  );
};

export default App;
