import React, { useState, useEffect, useRef } from 'react';
import { 
  Layout, Menu, Typography, Card, Table, Row, Col, Statistic, Button, 
  Input, Form, Switch, message, Dropdown, ConfigProvider, Alert, Radio, Tag, Badge, Space, Modal, Select, Checkbox
} from 'antd';
import { GlobalOutlined, UserOutlined, MailOutlined, MessageOutlined, LoginOutlined, LogoutOutlined, SettingOutlined, RobotOutlined, WarningOutlined, PlusOutlined, EditOutlined, DeleteOutlined } from '@ant-design/icons';
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
  '/finance': '6',
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
};

const Dashboard: React.FC = () => {
  const { t, i18n } = useTranslation();
  const { user } = useAuth();
  const isPrivileged = user?.role === 'admin' || user?.role === 'developer';
  const formatDateTimeLocal = (date: Date) => {
    const year = date.getFullYear();
    const month = String(date.getMonth() + 1).padStart(2, '0');
    const day = String(date.getDate()).padStart(2, '0');
    const hour = String(date.getHours()).padStart(2, '0');
    const minute = String(date.getMinutes()).padStart(2, '0');
    return `${year}-${month}-${day}T${hour}:${minute}`;
  };

  const defaultNowLocal = () => formatDateTimeLocal(new Date());

  const defaultRecentFrom = (days: number = 1) => {
    return formatDateTimeLocal(new Date(Date.now() - days * 24 * 60 * 60 * 1000));
  };

  const formatInUserTimezone = (value?: string) => {
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
  };
  const [personalEmails, setPersonalEmails] = useState<any[]>([]);
  const [globalStats, setGlobalStats] = useState<any>(null);
  const [personalStats, setPersonalStats] = useState<any>(null);
  const [mailErrors, setMailErrors] = useState<any[]>([]);
  const [logLevelFilter, setLogLevelFilter] = useState<string>('all');
  const [logTypeFilter, setLogTypeFilter] = useState<string>('all');
  const [logUserFilter, setLogUserFilter] = useState<string>('all');
  const [logKeyword, setLogKeyword] = useState<string>('');
  const [logTimeFrom, setLogTimeFrom] = useState<string>(defaultRecentFrom());
  const [logTimeTo, setLogTimeTo] = useState<string>(defaultNowLocal());
  const [feedbackRows, setFeedbackRows] = useState<any[]>([]);
  const [replyingFeedback, setReplyingFeedback] = useState<any | null>(null);
  const [replyText, setReplyText] = useState<string>('');
  const [replying, setReplying] = useState(false);

  const loadFeedback = async () => {
    if (!user?.email) return;
    try {
      const res = await axios.get(`/api/feedback?email=${encodeURIComponent(user.email)}`);
      setFeedbackRows(res.data.feedback || []);
    } catch {
      setFeedbackRows([]);
    }
  };

  const markFeedbackRead = async (feedbackId: number, isRead: boolean) => {
    if (!user?.email || !isPrivileged) return;
    try {
      await axios.post('/api/feedback/mark-read', {
        email: user.email,
        feedback_id: feedbackId,
        is_read: isRead,
      });
      loadFeedback();
    } catch {
      message.error('Failed to update feedback read state.');
    }
  };

  const submitFeedbackReply = async () => {
    if (!user?.email || !replyingFeedback || !replyText.trim()) return;
    setReplying(true);
    try {
      await axios.post('/api/feedback/reply', {
        email: user.email,
        feedback_id: replyingFeedback.id,
        reply_message: replyText.trim(),
      });
      message.success('Reply sent as AI assistant.');
      setReplyingFeedback(null);
      setReplyText('');
      loadFeedback();
    } catch {
      message.error('Failed to send feedback reply.');
    } finally {
      setReplying(false);
    }
  };

  useEffect(() => {
    const url = user ? `/api/dashboard?email=${user.email}` : `/api/dashboard`;
    axios.get(url).then(res => {
      setGlobalStats(res.data.global_stats);
      if (res.data.type === 'admin') {
        setPersonalEmails(res.data.personal_emails);
        setPersonalStats(res.data.personal_stats);
      } else if (res.data.type === 'personal') {
        setPersonalEmails(res.data.personal_emails);
        setPersonalStats(res.data.personal_stats);
      }
    });
    if (isPrivileged) {
      axios.get(`/api/admin/errors?email=${user.email}`).then(res => {
        setMailErrors(res.data.errors || []);
      }).catch(() => {});
    } else if (user?.role === 'user') {
      axios.get(`/api/errors?email=${user.email}`).then(res => {
        setMailErrors(res.data.errors || []);
      }).catch(() => {});
    }
    loadFeedback();
  }, [user, isPrivileged]);

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
      title: 'Rule Label',
      dataIndex: 'matched_rule_label',
      key: 'matched_rule_label',
      width: 140,
      render: (value?: string) => value ? <Tag color="blue">{value}</Tag> : '-',
    },
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
    unknown_sender: 'gold',
    parse_error: 'volcano',
  };

  const errorColumns = [
    {
      title: 'Level',
      dataIndex: 'level',
      key: 'level',
      width: 90,
      render: (v: string) => <Tag color={v === 'WARN' ? 'gold' : 'red'}>{v}</Tag>,
    },
    {
      title: 'Type',
      dataIndex: 'error_type',
      key: 'error_type',
      width: 120,
      render: (v: string) => <Tag color={errorTypeColor[v] || 'default'}>{v}</Tag>,
    },
    { title: 'User', dataIndex: 'user_email', key: 'user_email', width: 220, render: (v: string) => v || '-' },
    { title: 'Message', dataIndex: 'message', key: 'message', ellipsis: true },
    { title: 'Context', dataIndex: 'context', key: 'context', width: 180, ellipsis: true },
    {
      title: 'Time',
      dataIndex: 'occurred_at',
      key: 'occurred_at',
      width: 190,
      render: (value: string) => formatInUserTimezone(value),
    },
  ];

  const parseLogDate = (value?: string): Date | null => {
    if (!value) return null;
    const iso = value.includes('T') ? value : `${value.replace(' ', 'T')}Z`;
    const d = new Date(iso);
    return Number.isNaN(d.getTime()) ? null : d;
  };

  const logTypeOptions = Array.from(new Set(mailErrors.map((l) => l.error_type).filter(Boolean)));
  const logUserOptions = Array.from(new Set(mailErrors.map((l) => l.user_email).filter(Boolean)));

  const filteredMailErrors = mailErrors.filter((log) => {
    if (logLevelFilter !== 'all' && log.level !== logLevelFilter) return false;
    if (logTypeFilter !== 'all' && log.error_type !== logTypeFilter) return false;
    if (logUserFilter === '__unassigned__' && !!log.user_email) return false;
    if (logUserFilter !== 'all' && logUserFilter !== '__unassigned__' && log.user_email !== logUserFilter) return false;

    if (logKeyword.trim()) {
      const q = logKeyword.trim().toLowerCase();
      const haystack = [log.message, log.context, log.error_type, log.user_email]
        .filter(Boolean)
        .join(' ')
        .toLowerCase();
      if (!haystack.includes(q)) return false;
    }

    const logDate = parseLogDate(log.occurred_at);
    if (!logDate) return false;
    if (logTimeFrom) {
      const from = new Date(logTimeFrom);
      if (!Number.isNaN(from.getTime()) && logDate < from) return false;
    }
    if (logTimeTo) {
      const to = new Date(logTimeTo);
      if (!Number.isNaN(to.getTime()) && logDate > to) return false;
    }

    return true;
  });

  const LogFilterBar = () => (
    <div style={{ marginBottom: 12 }}>
      <Space wrap>
        <Space>
          <Button onClick={() => { setLogTimeFrom(defaultRecentFrom(1)); setLogTimeTo(defaultNowLocal()); }}>1d</Button>
          <Button onClick={() => { setLogTimeFrom(defaultRecentFrom(7)); setLogTimeTo(defaultNowLocal()); }}>7d</Button>
          <Button onClick={() => { setLogTimeFrom(defaultRecentFrom(30)); setLogTimeTo(defaultNowLocal()); }}>30d</Button>
        </Space>
        <Select
          value={logLevelFilter}
          style={{ width: 130 }}
          onChange={setLogLevelFilter}
          options={[
            { value: 'all', label: 'All Levels' },
            { value: 'ERROR', label: 'ERROR' },
            { value: 'WARN', label: 'WARN' },
          ]}
        />
        <Select
          value={logTypeFilter}
          style={{ width: 180 }}
          onChange={setLogTypeFilter}
          options={[
            { value: 'all', label: 'All Types' },
            ...logTypeOptions.map((v) => ({ value: v, label: v })),
          ]}
        />
        <Select
          value={logUserFilter}
          style={{ width: 220 }}
          onChange={setLogUserFilter}
          options={[
            { value: 'all', label: 'All Users' },
            { value: '__unassigned__', label: 'Unassigned' },
            ...logUserOptions.map((v) => ({ value: v, label: v })),
          ]}
        />
        <Input
          placeholder="Search message/type/context"
          value={logKeyword}
          onChange={(e) => setLogKeyword(e.target.value)}
          style={{ width: 260 }}
        />
        <Input
          type="datetime-local"
          value={logTimeFrom}
          onChange={(e) => setLogTimeFrom(e.target.value)}
          style={{ width: 210 }}
        />
        <Input
          type="datetime-local"
          value={logTimeTo}
          onChange={(e) => setLogTimeTo(e.target.value)}
          style={{ width: 210 }}
        />
        <Button
          onClick={() => {
            setLogLevelFilter('all');
            setLogTypeFilter('all');
            setLogUserFilter('all');
            setLogKeyword('');
            setLogTimeFrom(defaultRecentFrom());
            setLogTimeTo(defaultNowLocal());
          }}
        >
          Reset
        </Button>
      </Space>
    </div>
  );

  const feedbackColumns = [
    ...(isPrivileged ? [{ title: 'User', dataIndex: 'user_email', key: 'user_email', width: 220, render: (v: string) => v || '-' }] : []),
    {
      title: 'Rating',
      dataIndex: 'rating',
      key: 'rating',
      width: 90,
      render: (v: string) => <Tag color={v === 'up' ? 'green' : 'volcano'}>{v === 'up' ? '👍' : '👎'}</Tag>,
    },
    {
      title: 'Suggestion',
      dataIndex: 'suggestion',
      key: 'suggestion',
      render: (v: string) => v || '-',
    },
    {
      title: 'Read',
      dataIndex: 'is_read',
      key: 'is_read',
      width: 100,
      render: (v: boolean) => <Tag color={v ? 'green' : 'orange'}>{v ? 'Read' : 'Unread'}</Tag>,
    },
    {
      title: 'AI Reply',
      dataIndex: 'admin_reply',
      key: 'admin_reply',
      render: (v: string) => v || '-',
    },
    {
      title: 'Time',
      dataIndex: 'created_at',
      key: 'created_at',
      width: 170,
      render: (value: string) => formatInUserTimezone(value),
    },
    ...(isPrivileged ? [{
      title: 'Action',
      key: 'action',
      width: 260,
      render: (_: unknown, record: any) => (
        <Space>
          <Button size="small" onClick={() => markFeedbackRead(record.id, !record.is_read)}>
            {record.is_read ? 'Mark Unread' : 'Mark Read'}
          </Button>
          <Button
            size="small"
            type="primary"
            disabled={!record.user_email}
            onClick={() => {
              setReplyingFeedback(record);
              setReplyText(record.admin_reply || '');
            }}
          >
            Reply as AI
          </Button>
        </Space>
      ),
    }] : []),
  ];

  if (isPrivileged) {
    return (
      <div>
        <div style={{ marginBottom: 32 }}>
          <Title level={2}>{t('welcome')}, {user?.role === 'developer' ? 'Developer' : 'Admin'} ({user?.display_name || user?.email})</Title>
        </div>
        <GlobalStatsDisplay />
        <PersonalStatsDisplay />
        <Row gutter={[24, 24]}>
          <Col xs={24}>
            <Card bordered={false} title="Your Emails">
              <Table dataSource={personalEmails} rowKey="id" columns={columns} pagination={{ pageSize: 5 }} />
            </Card>
          </Col>
          <Col xs={24}>
            <Card
              bordered={false}
              title={
                <span>
                  <WarningOutlined style={{ color: '#ff4d4f', marginRight: 8 }} />
                  Mail Server Logs (Error + Warn)
                  {mailErrors.length > 0 && <Badge count={filteredMailErrors.length} style={{ marginLeft: 8, backgroundColor: '#ff4d4f' }} />}
                </span>
              }
            >
              <LogFilterBar />
              {mailErrors.length === 0
                ? <Alert message="No logs recorded." type="success" showIcon />
                : <Table dataSource={filteredMailErrors} rowKey="id" columns={errorColumns} pagination={{ pageSize: 10 }} size="small" />}
            </Card>
          </Col>
          <Col xs={24}>
            <Card bordered={false} title="User Feedback Suggestions">
              <Table dataSource={feedbackRows} rowKey="id" columns={feedbackColumns as any} pagination={{ pageSize: 8 }} size="small" />
            </Card>
          </Col>
        </Row>

        <Modal
          title="Reply to Feedback (sent as AI assistant)"
          open={!!replyingFeedback}
          onCancel={() => {
            setReplyingFeedback(null);
            setReplyText('');
          }}
          onOk={submitFeedbackReply}
          confirmLoading={replying}
        >
          <Input.TextArea
            rows={5}
            value={replyText}
            onChange={(e) => setReplyText(e.target.value)}
            placeholder="Type reply that will be emailed to the user from AI assistant mailbox"
          />
        </Modal>
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
        <Card bordered={false} title="Your Mail Server Logs" style={{ marginTop: 24 }}>
          <LogFilterBar />
          {mailErrors.length === 0
            ? <Alert message="No logs related to your account." type="success" showIcon />
            : <Table dataSource={filteredMailErrors} rowKey="id" columns={errorColumns} pagination={{ pageSize: 8 }} size="small" />}
        </Card>
        <Card bordered={false} title="Your Feedback Suggestions" style={{ marginTop: 24 }}>
          {feedbackRows.length === 0
            ? <Alert message="No feedback submitted yet." type="info" showIcon />
            : <Table dataSource={feedbackRows} rowKey="id" columns={feedbackColumns as any} pagination={{ pageSize: 8 }} size="small" />}
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
  const [deleting, setDeleting] = useState(false);
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
        training_data_consent: !!user.training_data_consent,
        timezone: user.timezone || 'UTC',
        preferred_language: user.preferred_language || 'en',
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

  const requestDataDeletion = async () => {
    if (!user) return;
    setDeleting(true);
    try {
      const res = await axios.post('/api/data-deletion/request', { email: user.email });
      if (res.data?.status === 'success') {
        message.success('Deletion request submitted. Please check your email for the confirmation link.');
      } else {
        message.error(res.data?.message || 'Failed to request data deletion.');
      }
    } catch {
      message.error('Failed to request data deletion.');
    } finally {
      setDeleting(false);
    }
  };

  return (
    <Card title={t('system_settings')} bordered={false} style={{ maxWidth: 650, margin: '0 auto' }}>
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

            <Title level={5} style={{ margin: '24px 0 16px' }}>{t('processing_preferences')}</Title>
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
              name="training_data_consent"
              label={t('training_data_consent')}
              valuePropName="checked"
              tooltip={t('training_data_consent_desc')}
            >
              <Switch />
            </Form.Item>
            <Paragraph style={{ color: '#86868b', fontSize: '12px', marginTop: -8 }}>
              {t('training_data_consent_note')}
            </Paragraph>

            <Form.Item
              name="preferred_language"
              label={t('preferred_language')}
              tooltip="Language used for system-generated emails sent to you."
              rules={[{ required: true, message: 'Please select a language' }]}
            >
              <Select
                options={[
                  { value: 'en', label: 'English' },
                  { value: 'zh-TW', label: '繁體中文' },
                ]}
              />
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

        {user && (
          <Card
            size="small"
            title="GDPR / Data Deletion"
            style={{ marginTop: 12, borderColor: '#ffd8bf' }}
          >
            <Paragraph style={{ marginBottom: 12, color: '#8c8c8c' }}>
              You can request deletion of all your data. The system will email you a confirmation link with a data summary report. After opening the link, you must confirm again before final deletion.
            </Paragraph>
            <Button danger loading={deleting} onClick={requestDataDeletion}>
              Request Deletion of All My Data
            </Button>
          </Card>
        )}
      </Form>
    </Card>
  );
};

const GdprDeletePage: React.FC = () => {
  const [loading, setLoading] = useState(true);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [summary, setSummary] = useState<any>(null);
  const [confirmed, setConfirmed] = useState(false);
  const location = useLocation();
  const token = new URLSearchParams(location.search).get('token') || '';

  useEffect(() => {
    if (!token) {
      setError('Missing token');
      setLoading(false);
      return;
    }

    axios.get(`/api/data-deletion/summary?token=${encodeURIComponent(token)}`)
      .then((res) => {
        if (res.data?.status === 'ready') {
          setSummary(res.data);
        } else {
          setError(res.data?.message || 'Invalid token');
        }
      })
      .catch(() => setError('Failed to load deletion summary.'))
      .finally(() => setLoading(false));
  }, [token]);

  const finalizeDeletion = async () => {
    if (!confirmed || !token) return;
    setSubmitting(true);
    try {
      const res = await axios.post('/api/data-deletion/confirm', { token, confirm: true });
      if (res.data?.status === 'success') {
        message.success('Your data has been deleted.');
        setSummary(null);
      } else {
        message.error(res.data?.message || 'Failed to delete data.');
      }
    } catch {
      message.error('Failed to delete data.');
    } finally {
      setSubmitting(false);
    }
  };

  if (loading) {
    return <Card bordered={false}><Paragraph>Loading deletion summary...</Paragraph></Card>;
  }
  if (error) {
    return <Alert type="error" showIcon message="Data Deletion" description={error} />;
  }
  if (!summary) {
    return <Alert type="success" showIcon message="Data Deletion Completed" description="No further action is required." />;
  }

  const s = summary.snapshot || {};

  return (
    <Card bordered={false} title="Confirm Data Deletion (Final Step)">
      <Alert
        type="warning"
        showIcon
        style={{ marginBottom: 16 }}
        message="Please review your data summary before final deletion"
        description={summary.warning}
      />
      <Table
        pagination={false}
        rowKey="key"
        dataSource={[
          { key: 'emails', item: 'Emails', value: s.email_count ?? 0 },
          { key: 'rules', item: 'Rules', value: s.rule_count ?? 0 },
          { key: 'logs', item: 'Logs', value: s.log_count ?? 0 },
          { key: 'memories', item: 'Memories', value: s.memory_count ?? 0 },
          { key: 'activities', item: 'Activity Rows', value: s.activity_row_count ?? 0 },
          { key: 'events', item: 'Activity Total Count', value: s.activity_event_total ?? 0 },
          { key: 'chat', item: 'Chat Logs', value: s.chat_log_count ?? 0 },
          { key: 'files', item: 'Files', value: s.file_count ?? 0 },
          { key: 'bytes', item: 'Total File Size (bytes)', value: s.total_file_bytes ?? 0 },
        ]}
        columns={[
          { title: 'Item', dataIndex: 'item', key: 'item' },
          { title: 'Value', dataIndex: 'value', key: 'value' },
        ]}
        style={{ marginBottom: 16 }}
      />

      <Checkbox checked={confirmed} onChange={(e) => setConfirmed(e.target.checked)}>
        I understand the deletion is irreversible and cached/overwritten database pages are not recoverable.
      </Checkbox>

      <div style={{ marginTop: 16 }}>
        <Button danger type="primary" disabled={!confirmed} loading={submitting} onClick={finalizeDeletion}>
          Confirm and Delete My Data
        </Button>
      </div>
    </Card>
  );
};

type EmailRule = {
  id: number;
  rule_text: string;
  rule_label: string;
  source: string;
  is_enabled: boolean;
  matched_count: number;
  updated_at?: string;
};

const RulesManager: React.FC = () => {
  const { t, i18n } = useTranslation();
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
      message.error(t('rules_load_failed'));
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
      message.success(t('rules_added'));
      loadRules();
    } catch {
      message.error(t('rules_add_failed'));
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
      message.error(t('rules_toggle_failed'));
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
      message.success(t('rules_updated'));
      loadRules();
    } catch {
      message.error(t('rules_update_failed'));
    } finally {
      setSaving(false);
    }
  };

  const deleteRule = async (rule: EmailRule) => {
    if (!user) return;
    const confirmed = window.confirm(`Delete rule #${rule.id}?\n${rule.rule_text}`);
    if (!confirmed) return;
    try {
      await axios.post('/api/rules/delete', { email: user.email, id: rule.id });
      message.success('Rule deleted');
      loadRules();
    } catch {
      message.error('Failed to delete rule');
    }
  };

  const columns = [
    {
      title: t('rules_rule'),
      dataIndex: 'rule_text',
      key: 'rule_text',
      render: (v: string) => <span>{v}</span>,
    },
    {
      title: 'Label',
      dataIndex: 'rule_label',
      key: 'rule_label',
      width: 150,
      render: (v: string) => <Tag color="blue">{v || 'RULE'}</Tag>,
    },
    {
      title: t('rules_source'),
      dataIndex: 'source',
      key: 'source',
      width: 120,
      render: (v: string) => <Tag color={v === 'chat' ? 'cyan' : 'default'}>{v}</Tag>,
    },
    {
      title: 'Matched',
      dataIndex: 'matched_count',
      key: 'matched_count',
      width: 110,
      render: (v: number) => v ?? 0,
    },
    {
      title: t('rules_enabled'),
      dataIndex: 'is_enabled',
      key: 'is_enabled',
      width: 120,
      render: (_: boolean, record: EmailRule) => (
        <Switch checked={record.is_enabled} onChange={(checked) => toggleRule(record, checked)} />
      ),
    },
    {
      title: t('rules_updated_at'),
      dataIndex: 'updated_at',
      key: 'updated_at',
      width: 200,
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
      },
    },
    {
      title: t('rules_action'),
      key: 'action',
      width: 220,
      render: (_: unknown, record: EmailRule) => (
        <Space>
          <Button
            icon={<EditOutlined />}
            onClick={() => {
              setEditingRule(record);
              setEditText(record.rule_text);
            }}
          >
            {t('rules_edit')}
          </Button>
          <Button danger icon={<DeleteOutlined />} onClick={() => deleteRule(record)}>
            Delete
          </Button>
        </Space>
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
    <Card bordered={false} title={t('rules_title')}>
      <Paragraph style={{ color: '#666' }}>
        {t('rules_desc')}
      </Paragraph>
      <Space.Compact style={{ width: '100%', marginBottom: 16 }}>
        <Input
          placeholder="Example: 如果是信用卡帳單，先摘要重點再提醒繳費期限"
          value={newRule}
          onChange={(e) => setNewRule(e.target.value)}
          onPressEnter={addRule}
        />
        <Button type="primary" icon={<PlusOutlined />} onClick={addRule} loading={saving}>
          {t('rules_add')}
        </Button>
      </Space.Compact>

      <Table rowKey="id" dataSource={rules} columns={columns} pagination={{ pageSize: 8 }} />

      <Modal
        title={t('rules_edit')}
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

type FinanceRecord = {
  id: string;
  subject?: string;
  reason: string;
  category: string;
  direction: string;
  amount: number;
  currency: string;
  month_key: string;
  month_total_after: number;
  created_at: string;
};

type MonthlyFinance = {
  month_key: string;
  category: string;
  total_amount: number;
  updated_at: string;
};

const FinanceAnalysisPage: React.FC = () => {
  const { user } = useAuth();
  const { i18n } = useTranslation();
  const [records, setRecords] = useState<FinanceRecord[]>([]);
  const [monthly, setMonthly] = useState<MonthlyFinance[]>([]);

  const formatInUserTimezone = (value?: string) => {
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
  };

  useEffect(() => {
    if (!user?.email) return;
    axios.get(`/api/finance/records?email=${encodeURIComponent(user.email)}`).then((res) => {
      setRecords(res.data.records || []);
    }).catch(() => setRecords([]));

    axios.get(`/api/finance/monthly?email=${encodeURIComponent(user.email)}`).then((res) => {
      setMonthly(res.data.monthly || []);
    }).catch(() => setMonthly([]));
  }, [user?.email]);

  if (!user) {
    return (
      <Card bordered={false}>
        <Alert type="info" showIcon message="Please login first" description="Finance analysis is available for logged-in users only." />
      </Card>
    );
  }

  const monthlyColumns = [
    { title: 'Month', dataIndex: 'month_key', key: 'month_key', width: 120 },
    { title: 'Category', dataIndex: 'category', key: 'category', width: 120, render: (v: string) => <Tag color="blue">{v}</Tag> },
    { title: 'Total Amount', dataIndex: 'total_amount', key: 'total_amount', width: 180, render: (v: number) => v?.toLocaleString() ?? '0' },
    { title: 'Updated At', dataIndex: 'updated_at', key: 'updated_at', width: 200, render: (v: string) => formatInUserTimezone(v) },
  ];

  const recordColumns = [
    { title: 'Time', dataIndex: 'created_at', key: 'created_at', width: 190, render: (v: string) => formatInUserTimezone(v) },
    { title: 'Subject', dataIndex: 'subject', key: 'subject', ellipsis: true },
    { title: 'Reason', dataIndex: 'reason', key: 'reason', ellipsis: true },
    { title: 'Category', dataIndex: 'category', key: 'category', width: 120, render: (v: string) => <Tag>{v}</Tag> },
    { title: 'Direction', dataIndex: 'direction', key: 'direction', width: 120, render: (v: string) => <Tag color={v === 'income' ? 'green' : 'volcano'}>{v}</Tag> },
    { title: 'Amount', dataIndex: 'amount', key: 'amount', width: 130, render: (v: number) => v?.toLocaleString() ?? '0' },
    { title: 'Currency', dataIndex: 'currency', key: 'currency', width: 100 },
    { title: 'Month', dataIndex: 'month_key', key: 'month_key', width: 100 },
    { title: 'Month Running Total', dataIndex: 'month_total_after', key: 'month_total_after', width: 180, render: (v: number) => v?.toLocaleString() ?? '0' },
  ];

  return (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Card bordered={false} title="Monthly Amount Summary">
        <Table rowKey={(r: MonthlyFinance) => `${r.month_key}-${r.category}`} columns={monthlyColumns as any} dataSource={monthly} pagination={{ pageSize: 12 }} />
      </Card>
      <Card bordered={false} title="Email Financial Analysis Records">
        <Table rowKey="id" columns={recordColumns as any} dataSource={records} pagination={{ pageSize: 10 }} />
      </Card>
    </Space>
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
  }, [location.search, user]);

  // Update document.title based on current page
  useEffect(() => {
    const titles: Record<string, string> = {
      '1': t('dashboard'),
      '2': t('ai_chat'),
      '3': t('settings'),
      '4': t('about'),
      '5': t('rules'),
      '6': 'Finance',
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
            <GdprDeletePage />
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
                { key: '5', label: t('rules') },
                { key: '6', label: 'Finance' },
                { key: '4', label: t('about') },
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
          {activeMenu === '5' && <RulesManager />}
          {activeMenu === '6' && <FinanceAnalysisPage />}
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
