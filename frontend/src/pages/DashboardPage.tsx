import React, { useEffect, useState } from 'react';
import {
  Alert,
  Badge,
  Button,
  Card,
  Col,
  Input,
  Modal,
  Row,
  Select,
  Space,
  Statistic,
  Table,
  Tag,
  Typography,
  message,
} from 'antd';
import { MailOutlined, MessageOutlined, RobotOutlined, UserOutlined, WarningOutlined } from '@ant-design/icons';
import axios from 'axios';
import { useTranslation } from 'react-i18next';
import { useAuth } from '../AuthContext';
import { GUEST_NAME_KEY } from '../Chat';

const { Title, Paragraph } = Typography;

const DashboardPage: React.FC = () => {
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
  const defaultRecentFrom = (days = 1) => formatDateTimeLocal(new Date(Date.now() - days * 24 * 60 * 60 * 1000));

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
    const url = user ? `/api/dashboard?email=${user.email}` : '/api/dashboard';
    axios.get(url).then((res) => {
      setGlobalStats(res.data.global_stats);
      if (res.data.type === 'admin' || res.data.type === 'personal') {
        setPersonalEmails(res.data.personal_emails);
        setPersonalStats(res.data.personal_stats);
      }
    });

    if (isPrivileged && user?.email) {
      axios.get(`/api/admin/errors?email=${user.email}`).then((res) => {
        setMailErrors(res.data.errors || []);
      }).catch(() => {});
    } else if (user?.role === 'user' && user.email) {
      axios.get(`/api/errors?email=${user.email}`).then((res) => {
        setMailErrors(res.data.errors || []);
      }).catch(() => {});
    }

    loadFeedback();
  }, [user, isPrivileged]);

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
      },
    },
    {
      title: 'Received At',
      dataIndex: 'received_at',
      key: 'received_at',
      render: (value: string) => formatInUserTimezone(value),
    },
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
      const haystack = [log.message, log.context, log.error_type, log.user_email].filter(Boolean).join(' ').toLowerCase();
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
        <Input placeholder="Search message/type/context" value={logKeyword} onChange={(e) => setLogKeyword(e.target.value)} style={{ width: 260 }} />
        <Input type="datetime-local" value={logTimeFrom} onChange={(e) => setLogTimeFrom(e.target.value)} style={{ width: 210 }} />
        <Input type="datetime-local" value={logTimeTo} onChange={(e) => setLogTimeTo(e.target.value)} style={{ width: 210 }} />
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

export default DashboardPage;
