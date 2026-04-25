import React, { useEffect, useMemo, useState } from 'react';
import {
  Alert,
  Badge,
  Button,
  Card,
  Col,
  Grid,
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
import { CloudServerOutlined, MailOutlined, MessageOutlined, RobotOutlined, UserOutlined, WarningOutlined } from '@ant-design/icons';
import axios from 'axios';
import { useTranslation } from 'react-i18next';
import { useLocation, useNavigate } from 'react-router-dom';
import { useAuth } from '../AuthContext';
import { GUEST_NAME_KEY } from '../Chat';

const { Title, Paragraph } = Typography;

const DashboardPage: React.FC = () => {
  const { t, i18n } = useTranslation();
  const { user } = useAuth();
  const navigate = useNavigate();
  const location = useLocation();
  const isPrivileged = user?.role === 'admin' || user?.role === 'developer';
  const screens = Grid.useBreakpoint();
  const isUltraWide = !!screens.xxl;

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
  const [selectedEmailRowKeys, setSelectedEmailRowKeys] = useState<React.Key[]>([]);
  const [processingSelected, setProcessingSelected] = useState(false);
  const [retryingErrorId, setRetryingErrorId] = useState<number | null>(null);
  const [statusFilter, setStatusFilter] = useState<string>('all');
  const [resultsModalVisible, setResultsModalVisible] = useState(false);
  const [resultsData, setResultsData] = useState<any>(null);
  const [draftRepliesByEmailId, setDraftRepliesByEmailId] = useState<Record<string, { id: string; body: string; from: string; subject: string }>>({});
  const [editingDraft, setEditingDraft] = useState<{ id: string; emailId: string; body: string; from: string; subject: string } | null>(null);
  const [draftEditorText, setDraftEditorText] = useState<string>('');
  const [draftEditorOpen, setDraftEditorOpen] = useState(false);
  const [savingDraft, setSavingDraft] = useState(false);
  const [sendingDraft, setSendingDraft] = useState(false);
  const [reprocessingEmailId, setReprocessingEmailId] = useState<string | null>(null);
  const [runtimeInfo, setRuntimeInfo] = useState<any>(null);

  const loadFeedback = async () => {
    if (!user?.email) return;
    try {
      const res = await axios.get(`/api/feedback?email=${encodeURIComponent(user.email)}`);
      setFeedbackRows(res.data.feedback || []);
    } catch {
      setFeedbackRows([]);
    }
  };

  const loadDraftReplies = async () => {
    if (!user?.email) {
      setDraftRepliesByEmailId({});
      return;
    }
    try {
      const res = await axios.get('/api/auto-replies', { params: { email: user.email } });
      const replies = (res.data?.replies || []) as Array<{ id: string; source_email_id?: string; body: string; from: string; subject: string }>;
      const mapped: Record<string, { id: string; body: string; from: string; subject: string }> = {};
      for (const r of replies) {
        if (r.source_email_id) {
          mapped[r.source_email_id] = { id: r.id, body: r.body, from: r.from, subject: r.subject };
        }
      }
      setDraftRepliesByEmailId(mapped);
    } catch {
      setDraftRepliesByEmailId({});
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
    reloadDashboard().catch(() => {
      setPersonalEmails([]);
    });

    if (isPrivileged && user?.email) {
      axios.get(`/api/admin/errors?email=${user.email}`).then((res) => {
        setMailErrors(res.data.errors || []);
      }).catch(() => {});
      axios.get('/api/about').then((res) => {
        setRuntimeInfo(res.data || null);
      }).catch(() => setRuntimeInfo(null));
    } else if (user?.role === 'user' && user.email) {
      axios.get(`/api/errors?email=${user.email}`).then((res) => {
        setMailErrors(res.data.errors || []);
      }).catch(() => {});
    }

    loadFeedback();
    loadDraftReplies();
  }, [user, isPrivileged]);

  const emailIdFromQuery = useMemo(() => {
    const params = new URLSearchParams(location.search);
    return params.get('emailId') || '';
  }, [location.search]);

  const reloadDashboard = async () => {
    const url = user ? `/api/dashboard?email=${user.email}` : '/api/dashboard';
    const res = await axios.get(url);
    setGlobalStats(res.data.global_stats);
    if (res.data.type === 'admin' || res.data.type === 'personal') {
      setPersonalEmails(res.data.personal_emails || []);
      setPersonalStats(res.data.personal_stats);
    }
  };

  const filteredEmails = statusFilter === 'all'
    ? personalEmails
    : personalEmails.filter((e: any) => e.status === statusFilter);

  const columns = [
    { title: t('col_subject'), dataIndex: 'subject', key: 'subject' },
    {
      title: t('col_rule_label'),
      dataIndex: 'matched_rule_label',
      key: 'matched_rule_label',
      width: 140,
      render: (value?: string) => value ? <Tag color="blue">{value}</Tag> : '-',
    },
    {
      title: t('col_status', { defaultValue: 'Status' }),
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
      title: t('col_received_at'),
      dataIndex: 'received_at',
      key: 'received_at',
      render: (value: string) => formatInUserTimezone(value),
    },
    {
      title: t('col_action'),
      key: 'action',
      width: 320,
      render: (_: unknown, record: any) => (
        <Space wrap>
          <Button size="small" onClick={() => navigate(`/finance?emailId=${encodeURIComponent(record.id)}&subject=${encodeURIComponent(record.subject || '')}`)}>
            {t('view_finance')}
          </Button>
          <Button
            size="small"
            loading={reprocessingEmailId === record.id}
            onClick={() => reprocessSingleEmail(record.id)}
          >
            {t('btn_reprocess')}
          </Button>
          {record.status === 'drafted' && (
            <Button size="small" type="primary" onClick={() => openDraftEditor(record.id)}>
              {t('btn_view_edit_draft')}
            </Button>
          )}
        </Space>
      ),
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
      title: t('col_level'),
      dataIndex: 'level',
      key: 'level',
      width: 90,
      render: (v: string) => <Tag color={v === 'WARN' ? 'gold' : 'red'}>{v}</Tag>,
    },
    {
      title: t('col_type'),
      dataIndex: 'error_type',
      key: 'error_type',
      width: 120,
      render: (v: string) => <Tag color={errorTypeColor[v] || 'default'}>{v}</Tag>,
    },
    { title: t('col_user'), dataIndex: 'user_email', key: 'user_email', width: 220, render: (v: string) => v || '-' },
    { title: t('col_message'), dataIndex: 'message', key: 'message', ellipsis: true },
    { title: t('col_context'), dataIndex: 'context', key: 'context', width: 180, ellipsis: true },
    {
      title: t('col_time'),
      dataIndex: 'occurred_at',
      key: 'occurred_at',
      width: 190,
      render: (value: string) => formatInUserTimezone(value),
    },
    {
      title: t('col_action'),
      key: 'action',
      width: 120,
      render: (_: unknown, record: any) => {
        const canRetry = record.error_type === 'unknown_sender' && !!record.context;
        if (!canRetry) return '-';
        return (
          <Button
            size="small"
            loading={retryingErrorId === record.id}
            onClick={async () => {
              if (!user?.email) return;
              setRetryingErrorId(record.id);
              try {
                const res = await axios.post('/api/errors/retry', {
                  email: user.email,
                  error_id: record.id,
                });
                if (res.data?.status === 'success') {
                  message.success(res.data?.message || 'Queued for retry. The spool worker will reprocess it shortly.');
                } else {
                  message.error(res.data?.message || 'Retry failed.');
                }

                const url = isPrivileged
                  ? `/api/admin/errors?email=${encodeURIComponent(user.email)}`
                  : `/api/errors?email=${encodeURIComponent(user.email)}`;
                const refreshed = await axios.get(url);
                setMailErrors(refreshed.data.errors || []);
              } catch {
                message.error('Retry failed.');
              } finally {
                setRetryingErrorId(null);
              }
            }}
          >
            Recheck Delivered-To
          </Button>
        );
      },
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
    ...(isPrivileged ? [{ title: t('col_user'), dataIndex: 'user_email', key: 'user_email', width: 220, render: (v: string) => v || '-' }] : []),
    {
      title: t('col_rating'),
      dataIndex: 'rating',
      key: 'rating',
      width: 90,
      render: (v: string) => <Tag color={v === 'up' ? 'green' : 'volcano'}>{v === 'up' ? '👍' : '👎'}</Tag>,
    },
    {
      title: t('col_suggestion'),
      dataIndex: 'suggestion',
      key: 'suggestion',
      render: (v: string) => v || '-',
    },
    {
      title: t('col_read'),
      dataIndex: 'is_read',
      key: 'is_read',
      width: 100,
      render: (v: boolean) => <Tag color={v ? 'green' : 'orange'}>{v ? t('col_read') : t('col_unread', { defaultValue: 'Unread' })}</Tag>,
    },
    {
      title: t('col_ai_reply'),
      dataIndex: 'admin_reply',
      key: 'admin_reply',
      render: (v: string) => v || '-',
    },
    {
      title: t('col_time'),
      dataIndex: 'created_at',
      key: 'created_at',
      width: 170,
      render: (value: string) => formatInUserTimezone(value),
    },
    ...(isPrivileged ? [{
      title: t('col_action'),
      key: 'action',
      width: 260,
      render: (_: unknown, record: any) => (
        <Space>
          <Button size="small" onClick={() => markFeedbackRead(record.id, !record.is_read)}>
            {record.is_read ? t('btn_mark_unread') : t('btn_mark_read')}
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
            {t('btn_reply_as_ai')}
          </Button>
        </Space>
      ),
    }] : []),
  ];

  const processSelectedEmails = async () => {
    if (!user?.email || selectedEmailRowKeys.length === 0) return;
    setProcessingSelected(true);
    try {
      const emailIds = selectedEmailRowKeys.map((id) => String(id));
      const res = await axios.post('/api/emails/process-manual', {
        email: user.email,
        email_ids: emailIds,
      });
      setResultsData(res.data);
      setResultsModalVisible(true);
      setSelectedEmailRowKeys([]);
      await reloadDashboard();
      await loadDraftReplies();
    } catch {
      message.error(t('process_selected_failed'));
    } finally {
      setProcessingSelected(false);
    }
  };

  const reprocessSingleEmail = async (emailId: string) => {
    if (!user?.email) return;
    setReprocessingEmailId(emailId);
    try {
      const res = await axios.post('/api/emails/process-manual', {
        email: user.email,
        email_ids: [emailId],
        force_reextract: true,
      });
      const result = res.data?.results?.[0];
      if (result?.result === 'processed') {
        message.success('Reprocessed successfully.');
      } else {
        message.info(result?.reason || 'Reprocess queued.');
      }
      await reloadDashboard();
      await loadDraftReplies();
    } catch {
      message.error('Failed to reprocess this email.');
    } finally {
      setReprocessingEmailId(null);
    }
  };

  const openDraftEditor = (emailId: string) => {
    const draft = draftRepliesByEmailId[emailId];
    if (!draft) {
      message.warning('No draft found for this email yet.');
      return;
    }
    setEditingDraft({ id: draft.id, emailId, body: draft.body, from: draft.from, subject: draft.subject });
    setDraftEditorText(draft.body || '');
    setDraftEditorOpen(true);
  };

  const saveDraftChanges = async () => {
    if (!user?.email || !editingDraft) return false;
    setSavingDraft(true);
    try {
      await axios.post('/api/auto-replies/update', {
        email: user.email,
        reply_id: editingDraft.id,
        reply_body: draftEditorText,
      });
      message.success('Draft saved.');
      await loadDraftReplies();
      return true;
    } catch {
      message.error('Failed to save draft.');
      return false;
    } finally {
      setSavingDraft(false);
    }
  };

  const sendDraftNow = async () => {
    if (!user?.email || !editingDraft) return;
    setSendingDraft(true);
    try {
      const saved = await saveDraftChanges();
      if (!saved) return;
      await axios.post('/api/auto-replies/send', {
        email: user.email,
        reply_id: editingDraft.id,
      });
      message.success('Draft sent.');
      setDraftEditorOpen(false);
      setEditingDraft(null);
      setDraftEditorText('');
      await reloadDashboard();
      await loadDraftReplies();
    } catch {
      message.error('Failed to send draft.');
    } finally {
      setSendingDraft(false);
    }
  };

  const LogFilterBar = () => (
    <div style={{ marginBottom: 12 }}>
      <Space wrap>
        <Space>
          <Button onClick={() => { setLogTimeFrom(defaultRecentFrom(1)); setLogTimeTo(defaultNowLocal()); }}>{t('log_quick_1d')}</Button>
          <Button onClick={() => { setLogTimeFrom(defaultRecentFrom(7)); setLogTimeTo(defaultNowLocal()); }}>{t('log_quick_7d')}</Button>
          <Button onClick={() => { setLogTimeFrom(defaultRecentFrom(30)); setLogTimeTo(defaultNowLocal()); }}>{t('log_quick_30d')}</Button>
        </Space>
        <Select
          value={logLevelFilter}
          style={{ width: 130 }}
          onChange={setLogLevelFilter}
          options={[
            { value: 'all', label: t('log_all_levels') },
            { value: 'ERROR', label: 'ERROR' },
            { value: 'WARN', label: 'WARN' },
          ]}
        />
        <Select
          value={logTypeFilter}
          style={{ width: 180 }}
          onChange={setLogTypeFilter}
          options={[
            { value: 'all', label: t('log_all_types') },
            ...logTypeOptions.map((v) => ({ value: v, label: v })),
          ]}
        />
        <Select
          value={logUserFilter}
          style={{ width: 220 }}
          onChange={setLogUserFilter}
          options={[
            { value: 'all', label: t('log_all_users') },
            { value: '__unassigned__', label: t('log_unassigned') },
            ...logUserOptions.map((v) => ({ value: v, label: v })),
          ]}
        />
        <Input placeholder={t('log_search_placeholder')} value={logKeyword} onChange={(e) => setLogKeyword(e.target.value)} style={{ width: 260 }} />
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
          {t('log_reset')}
        </Button>
      </Space>
    </div>
  );

  const GlobalStatsDisplay = () => (
    globalStats ? (
      <div style={{ marginBottom: isUltraWide ? 0 : 32 }}>
        <Title level={4}>{t('dashboard_system_overview')}</Title>
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
      <div style={{ marginBottom: isUltraWide ? 0 : 32 }}>
        <Title level={4}>{t('dashboard_your_processing')}</Title>
        <Row gutter={[16, 16]}>
          <Col xs={12} sm={8} md={6}>
            <Card bordered={false} hoverable styles={{ body: { padding: '16px' } }}>
              <Statistic title={t('col_your_received')} value={personalStats.emails_received} prefix={<MailOutlined style={{ color: '#34c759' }} />} />
            </Card>
          </Col>
          <Col xs={12} sm={8} md={6}>
            <Card bordered={false} hoverable styles={{ body: { padding: '16px' } }}>
              <Statistic title={t('col_your_replied')} value={personalStats.emails_replied} prefix={<MessageOutlined style={{ color: '#ff9500' }} />} />
            </Card>
          </Col>
        </Row>
      </div>
    ) : null
  );

  const RemoteDebugDisplay = () => {
    if (!runtimeInfo) return null;
    const enabled = !!runtimeInfo.remote_debug_sshfs_enabled;
    const mode = runtimeInfo.remote_debug_mode || 'readonly';
    const isOverlay = mode === 'overlay';
    return (
      <Card
        bordered={false}
        title={
          <span>
            <CloudServerOutlined style={{ color: '#1677ff', marginRight: 8 }} />
            {t('remote_debug_title')}
          </span>
        }
      >
        <Space direction="vertical" style={{ width: '100%' }}>
          <Space wrap>
            <Tag color={enabled ? 'green' : 'default'}>{enabled ? t('remote_debug_enabled') : t('remote_debug_disabled')}</Tag>
            <Tag color={isOverlay ? 'blue' : 'gold'}>{isOverlay ? t('remote_debug_overlay') : t('remote_debug_readonly')}</Tag>
            {runtimeInfo.readonly_mode_enabled && <Tag color="orange">{t('remote_debug_app_readonly')}</Tag>}
          </Space>
          <div style={{ color: '#86868b' }}>{t('remote_debug_desc')}</div>
          <div style={{ display: 'grid', gridTemplateColumns: '120px 1fr', rowGap: 8, columnGap: 12 }}>
            <span>{t('remote_debug_remote')}</span>
            <code>{runtimeInfo.remote_debug_remote || '-'}</code>
            <span>{t('remote_debug_mount')}</span>
            <code>{runtimeInfo.remote_debug_mount_point || '-'}</code>
            <span>{t('remote_debug_overlay_dir')}</span>
            <code>{runtimeInfo.remote_debug_overlay_dir || runtimeInfo.overlay_dir || '-'}</code>
          </div>
        </Space>
      </Card>
    );
  };

  const ResultsModal = () => {
    if (!resultsData) return null;
    const { processed, skipped, failed, results } = resultsData;
    return (
      <Modal
        title={t('process_results')}
        open={resultsModalVisible}
        onCancel={() => setResultsModalVisible(false)}
        footer={[
          <Button key="close" type="primary" onClick={() => setResultsModalVisible(false)}>
            {t('close')}
          </Button>,
        ]}
      >
        <Space direction="vertical" style={{ width: '100%' }}>
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: '16px' }}>
            <Card bordered={false} hoverable>
              <div style={{ textAlign: 'center' }}>
                <div style={{ fontSize: '24px', fontWeight: 'bold', color: '#34c759' }}>{processed}</div>
                <div style={{ color: '#86868b', fontSize: '12px' }}>{t('processed_count')}</div>
              </div>
            </Card>
            <Card bordered={false} hoverable>
              <div style={{ textAlign: 'center' }}>
                <div style={{ fontSize: '24px', fontWeight: 'bold', color: '#ff9500' }}>{skipped}</div>
                <div style={{ color: '#86868b', fontSize: '12px' }}>{t('skipped_count')}</div>
              </div>
            </Card>
            <Card bordered={false} hoverable>
              <div style={{ textAlign: 'center' }}>
                <div style={{ fontSize: '24px', fontWeight: 'bold', color: '#ff3b30' }}>{failed}</div>
                <div style={{ color: '#86868b', fontSize: '12px' }}>{t('failed_count')}</div>
              </div>
            </Card>
          </div>
          <div>
            <Title level={5}>{t('details')}</Title>
            <Table
              dataSource={results || []}
              columns={[
                { title: t('email_id'), dataIndex: 'email_id', key: 'email_id', width: '50%', render: (v) => <code style={{ fontSize: '11px' }}>{v?.substring(0, 8)}...</code> },
                { title: t('result'), dataIndex: 'result', key: 'result', width: '25%', render: (v) => <Tag color={v === 'processed' ? 'green' : v === 'skipped' ? 'gold' : 'red'}>{v}</Tag> },
                { title: t('reason'), dataIndex: 'reason', key: 'reason', width: '25%', render: (v) => v || '-' },
              ]}
              pagination={false}
              size="small"
            />
          </div>
        </Space>
      </Modal>
    );
  };

  const DraftEditorModal = () => (
    <Modal
      title={editingDraft ? `${t('draft_modal_title')} - ${editingDraft.subject || '(no subject)'}` : t('draft_modal_title')}
      open={draftEditorOpen}
      onCancel={() => {
        setDraftEditorOpen(false);
        setEditingDraft(null);
        setDraftEditorText('');
      }}
      footer={[
        <Button key="cancel" onClick={() => {
          setDraftEditorOpen(false);
          setEditingDraft(null);
          setDraftEditorText('');
        }}>
          {t('btn_cancel')}
        </Button>,
        <Button key="save" onClick={saveDraftChanges} loading={savingDraft}>
          {t('btn_save_draft')}
        </Button>,
        <Button key="send" type="primary" onClick={sendDraftNow} loading={sendingDraft}>
          {t('btn_send_now')}
        </Button>,
      ]}
    >
      {editingDraft && (
        <>
          <Paragraph style={{ marginBottom: 8, color: '#86868b' }}>To: {editingDraft.from}</Paragraph>
          <Input.TextArea rows={10} value={draftEditorText} onChange={(e) => setDraftEditorText(e.target.value)} />
        </>
      )}
    </Modal>
  );

  const EmailTableWithFilter = () => (
    <Card bordered={false} title={t('your_emails')}>
      <Space direction="vertical" style={{ width: '100%', marginBottom: 12 }}>
        <Space wrap>
          <Button
            type={statusFilter === 'all' ? 'primary' : 'default'}
            onClick={() => setStatusFilter('all')}
          >
            {t('filter_all')}
          </Button>
          <Button
            type={statusFilter === 'pending' ? 'primary' : 'default'}
            onClick={() => setStatusFilter('pending')}
          >
            {t('filter_pending')}
          </Button>
          <Button
            type={statusFilter === 'drafted' ? 'primary' : 'default'}
            onClick={() => setStatusFilter('drafted')}
          >
            {t('filter_drafted')}
          </Button>
          <Button
            type={statusFilter === 'replied' ? 'primary' : 'default'}
            onClick={() => setStatusFilter('replied')}
          >
            {t('filter_replied')}
          </Button>
        </Space>
        <Space>
          <Button
            type="primary"
            disabled={selectedEmailRowKeys.length === 0}
            loading={processingSelected}
            onClick={processSelectedEmails}
          >
            {t('process_with_ai')}
          </Button>
          <span style={{ color: '#86868b' }}>
            {t('process_selected')}: {selectedEmailRowKeys.length}
          </span>
        </Space>
      </Space>
      <Table
        dataSource={filteredEmails}
        rowKey="id"
        columns={columns}
        scroll={{ x: 'max-content' }}
        pagination={{ pageSize: 5 }}
        rowSelection={{
          selectedRowKeys: selectedEmailRowKeys,
          onChange: (keys) => setSelectedEmailRowKeys(keys),
          getCheckboxProps: (record: any) => ({
            disabled: record.status !== 'pending',
          }),
        }}
        rowClassName={(record: any) => (emailIdFromQuery && record.id === emailIdFromQuery ? 'finance-linked-row' : '')}
      />
    </Card>
  );

  if (isPrivileged) {
    return (
      <div>
        <div style={{ marginBottom: 32 }}>
          <Title level={2}>{t('welcome')}, {user?.role === 'developer' ? 'Developer' : 'Admin'} ({user?.display_name || user?.email})</Title>
        </div>
        <Row gutter={[24, 24]} style={{ marginBottom: 24 }}>
          <Col xs={24} xxl={12}>
            <GlobalStatsDisplay />
          </Col>
          <Col xs={24} xxl={12}>
            <PersonalStatsDisplay />
          </Col>
        </Row>
        <Row gutter={[24, 24]}>
          <Col xs={24} xxl={14}>
            <EmailTableWithFilter />
          </Col>
          <Col xs={24} xxl={10}>
            <Space direction="vertical" size={24} style={{ width: '100%' }}>
              <Card
                bordered={false}
                title={
                  <span>
                    <WarningOutlined style={{ color: '#ff4d4f', marginRight: 8 }} />
                    {t('dashboard_mail_logs_admin')}
                    {mailErrors.length > 0 && <Badge count={filteredMailErrors.length} style={{ marginLeft: 8, backgroundColor: '#ff4d4f' }} />}
                  </span>
                }
              >
                <LogFilterBar />
                {mailErrors.length === 0
                  ? <Alert message={t('dashboard_no_logs')} type="success" showIcon />
                  : <Table dataSource={filteredMailErrors} rowKey="id" columns={errorColumns} scroll={{ x: 'max-content' }} pagination={{ pageSize: 10 }} size="small" />}
              </Card>
              <Card bordered={false} title={t('dashboard_feedback_admin')}>
                <Table dataSource={feedbackRows} rowKey="id" columns={feedbackColumns as any} scroll={{ x: 'max-content' }} pagination={{ pageSize: 8 }} size="small" />
              </Card>
              <RemoteDebugDisplay />
            </Space>
          </Col>
        </Row>

        <Modal
          title={t('feedback_reply_modal_title')}
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
            placeholder={t('feedback_reply_placeholder')}
          />
        </Modal>

        <ResultsModal />
        <DraftEditorModal />
      </div>
    );
  }

  if (user?.role === 'user') {
    return (
      <div>
        <div style={{ marginBottom: 32 }}>
          <Title level={2}>{t('welcome')}, {user.display_name || user.email}</Title>
        </div>
        <Row gutter={[24, 24]} style={{ marginBottom: 24 }}>
          <Col xs={24} xxl={12}>
            <GlobalStatsDisplay />
          </Col>
          <Col xs={24} xxl={12}>
            <PersonalStatsDisplay />
          </Col>
        </Row>
        <Row gutter={[24, 24]}>
          <Col xs={24} xxl={14}>
            <EmailTableWithFilter />
          </Col>
          <Col xs={24} xxl={10}>
            <Space direction="vertical" size={24} style={{ width: '100%' }}>
              <Card bordered={false} title={t('dashboard_mail_logs_user')}>
                <LogFilterBar />
                {mailErrors.length === 0
                  ? <Alert message={t('dashboard_no_logs_user')} type="success" showIcon />
                  : <Table dataSource={filteredMailErrors} rowKey="id" columns={errorColumns} scroll={{ x: 'max-content' }} pagination={{ pageSize: 8 }} size="small" />}
              </Card>
              <Card bordered={false} title={t('dashboard_feedback_user')}>
                {feedbackRows.length === 0
                  ? <Alert message={t('dashboard_no_feedback')} type="info" showIcon />
                  : <Table dataSource={feedbackRows} rowKey="id" columns={feedbackColumns as any} scroll={{ x: 'max-content' }} pagination={{ pageSize: 8 }} size="small" />}
              </Card>
            </Space>
          </Col>
        </Row>

        <ResultsModal />
        <DraftEditorModal />
      </div>
    );
  }

  const guestName = localStorage.getItem(GUEST_NAME_KEY);
  return (
    <div>
      <div style={{ marginBottom: 32 }}>
        <Title level={2}>{t('welcome')}{guestName ? `, ${guestName}` : ''}</Title>
        <Paragraph style={{ color: '#86868b', fontSize: '16px' }}>{t('dashboard_guest_desc')}</Paragraph>
      </div>
      <GlobalStatsDisplay />
      <div style={{ textAlign: 'center', marginTop: 60 }}>
        <Paragraph style={{ color: '#86868b', fontSize: '16px' }}>{t('dashboard_login_prompt')}</Paragraph>
      </div>
    </div>
  );
};

export default DashboardPage;
