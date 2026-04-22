import React, { useEffect, useState } from 'react';
import { Alert, Button, Card, Input, Modal, Space, Switch, Table, Tag, Typography, message } from 'antd';
import { DeleteOutlined, EditOutlined, PlusOutlined } from '@ant-design/icons';
import axios from 'axios';
import { useTranslation } from 'react-i18next';
import { useAuth } from '../AuthContext';

const { Paragraph } = Typography;

type EmailRule = {
  id: number;
  rule_text: string;
  rule_label: string;
  source: string;
  is_enabled: boolean;
  matched_count: number;
  updated_at?: string;
};

const deriveRuleName = (rule: EmailRule) => {
  const rawLabel = (rule.rule_label || '').trim();
  const strippedLabel = rawLabel.replace(/^RULE-/i, '').trim();
  if (strippedLabel) {
    return strippedLabel.slice(0, 18);
  }

  const text = (rule.rule_text || '').trim();
  if (!text) return 'RULE';

  const cjkChars = text.match(/[\u3400-\u9FFF]/g);
  if (cjkChars && cjkChars.length > 0) {
    return cjkChars.join('').slice(0, 8);
  }

  const words = text
    .split(/\s+/)
    .map((w) => w.replace(/[^\w-]/g, ''))
    .filter(Boolean)
    .slice(0, 3);

  return words.join(' ').slice(0, 24) || 'RULE';
};

const RulesManagerPage: React.FC = () => {
  const { t, i18n } = useTranslation();
  const { user } = useAuth();
  const [rules, setRules] = useState<EmailRule[]>([]);
  const [newRule, setNewRule] = useState('');
  const [saving, setSaving] = useState(false);
  const [editingRule, setEditingRule] = useState<EmailRule | null>(null);
  const [editText, setEditText] = useState('');
  const [confirmDeleteId, setConfirmDeleteId] = useState<number | null>(null);

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

  const deleteRule = async (id: number) => {
    if (!user) return;
    try {
      await axios.post('/api/rules/delete', { email: user.email, id });
      setConfirmDeleteId(null);
      message.success('Rule deleted');
      loadRules();
    } catch {
      message.error('Failed to delete rule');
    }
  };

  const columns = [
    {
      title: t('rules_rule'),
      dataIndex: 'rule_label',
      key: 'rule_label',
      width: 180,
      render: (_: string, record: EmailRule) => {
        const name = deriveRuleName(record);
        return <Tag color="blue">{name}</Tag>;
      },
    },
    {
      title: 'Details',
      dataIndex: 'rule_text',
      key: 'rule_text',
      ellipsis: true,
      render: (v: string) => (
        <Paragraph
          style={{ marginBottom: 0 }}
          ellipsis={{ rows: 2, expandable: false, tooltip: v }}
        >
          {v}
        </Paragraph>
      ),
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
          {confirmDeleteId === record.id ? (
            <Space>
              <span style={{ color: '#cf1322', fontSize: 13 }}>確定刪除?</span>
              <Button size="small" danger onClick={() => deleteRule(record.id)}>確定</Button>
              <Button size="small" onClick={() => setConfirmDeleteId(null)}>取消</Button>
            </Space>
          ) : (
            <Button danger icon={<DeleteOutlined />} onClick={() => setConfirmDeleteId(record.id)}>
              Delete
            </Button>
          )}
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

      <Table rowKey="id" dataSource={rules} columns={columns} scroll={{ x: 'max-content' }} pagination={{ pageSize: 8 }} />

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

export default RulesManagerPage;
