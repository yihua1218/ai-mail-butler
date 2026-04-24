import React, { useEffect, useMemo, useState } from 'react';
import { Alert, Button, Card, Space, Table, Tag } from 'antd';
import axios from 'axios';
import { useTranslation } from 'react-i18next';
import { useLocation, useNavigate } from 'react-router-dom';
import { useAuth } from '../AuthContext';

type FinanceRecord = {
  id: string;
  email_id: string;
  subject?: string;
  reason: string;
  category: string;
  direction: string;
  amount: number;
  currency: string;
  month_key: string;
  month_total_after: number;
  finance_type?: string;
  due_date?: string;
  statement_amount?: number;
  issuing_bank?: string;
  card_last4?: string;
  transaction_month_key?: string;
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
  const { i18n, t } = useTranslation();
  const navigate = useNavigate();
  const location = useLocation();
  const [records, setRecords] = useState<FinanceRecord[]>([]);
  const [monthly, setMonthly] = useState<MonthlyFinance[]>([]);

  const formatInUserTimezone = (value?: string) => {
    if (!value) return '-';
    const timezone = user?.timezone || 'UTC';
    const timeFormat = user?.time_format || '24h';
    const dateFormat = user?.date_format || 'auto';
    const iso = value.includes('T') ? value : `${value.replace(' ', 'T')}Z`;
    const date = new Date(iso);
    if (Number.isNaN(date.getTime())) return value;
    const localeMap: Record<string, string> = {
      iso: 'en-CA',
      us: 'en-US',
      eu: 'fr-FR',
      tw: 'zh-TW',
      auto: i18n.language === 'zh-TW' ? 'zh-TW' : 'en-US',
    };
    const locale = localeMap[dateFormat] ?? (i18n.language === 'zh-TW' ? 'zh-TW' : 'en-US');
    return new Intl.DateTimeFormat(locale, {
      timeZone: timezone,
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
      hour12: timeFormat === '12h',
    }).format(date);
  };

  const emailIdFromQuery = useMemo(() => {
    const params = new URLSearchParams(location.search);
    return params.get('emailId') || '';
  }, [location.search]);

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
        <Alert type="info" showIcon message={t('finance_login_required')} description={t('finance_login_desc')} />
      </Card>
    );
  }

  const monthlyCategoryColor = (category: string) => {
    if (category === 'deposit' || category === 'income') return 'green';
    if (category === 'expense') return 'volcano';
    return 'blue';
  };

  const monthlyColumns = [
    { title: t('finance_month_col'), dataIndex: 'month_key', key: 'month_key', width: 120 },
    { title: t('finance_category_col'), dataIndex: 'category', key: 'category', width: 120, render: (v: string) => <Tag color={monthlyCategoryColor(v)}>{t(`finance_cat_${v}`, { defaultValue: v })}</Tag> },
    { title: t('finance_total_amount_col'), dataIndex: 'total_amount', key: 'total_amount', width: 180, render: (v: number) => v?.toLocaleString() ?? '0' },
    { title: t('finance_updated_at_col'), dataIndex: 'updated_at', key: 'updated_at', width: 200, render: (v: string) => <span style={{ whiteSpace: 'nowrap' }}>{formatInUserTimezone(v)}</span> },
  ];

  const filteredRecords = emailIdFromQuery
    ? records.filter((row) => row.email_id === emailIdFromQuery)
    : records;

  const recordColumns = [
    { title: t('finance_time_col'), dataIndex: 'created_at', key: 'created_at', width: 190, render: (v: string) => <span style={{ whiteSpace: 'nowrap' }}>{formatInUserTimezone(v)}</span> },
    { title: t('finance_subject_col'), dataIndex: 'subject', key: 'subject', ellipsis: true },
    { title: t('finance_reason_col'), dataIndex: 'reason', key: 'reason', ellipsis: true },
    { title: t('finance_type'), dataIndex: 'finance_type', key: 'finance_type', width: 120, render: (v?: string) => v ? <Tag color={v === 'bill' ? 'blue' : 'purple'}>{t(`finance_cat_${v}`, { defaultValue: v })}</Tag> : '-' },
    { title: t('finance_category_col'), dataIndex: 'category', key: 'category', width: 120, render: (v: string) => <Tag>{t(`finance_cat_${v}`, { defaultValue: v })}</Tag> },
    { title: t('finance_direction_col'), dataIndex: 'direction', key: 'direction', width: 120, render: (v: string) => <Tag color={v === 'income' ? 'green' : 'volcano'}>{t(`finance_dir_${v}`, { defaultValue: v })}</Tag> },
    { title: t('finance_amount_col'), dataIndex: 'amount', key: 'amount', width: 130, render: (v: number) => v?.toLocaleString() ?? '0' },
    { title: t('statement_amount'), dataIndex: 'statement_amount', key: 'statement_amount', width: 150, render: (v?: number) => (typeof v === 'number' ? v.toLocaleString() : '-') },
    { title: t('due_date'), dataIndex: 'due_date', key: 'due_date', width: 130, render: (v?: string) => v || '-' },
    { title: t('issuing_bank'), dataIndex: 'issuing_bank', key: 'issuing_bank', width: 140, render: (v?: string) => v || '-' },
    { title: t('card_last4'), dataIndex: 'card_last4', key: 'card_last4', width: 120, render: (v?: string) => v || '-' },
    { title: t('finance_currency_col'), dataIndex: 'currency', key: 'currency', width: 100 },
    { title: t('finance_month_col'), dataIndex: 'month_key', key: 'month_key', width: 100 },
    { title: t('transaction_month_key'), dataIndex: 'transaction_month_key', key: 'transaction_month_key', width: 120, render: (v?: string) => v || '-' },
    { title: t('finance_month_running_total_col'), dataIndex: 'month_total_after', key: 'month_total_after', width: 180, render: (v: number) => v?.toLocaleString() ?? '0' },
    {
      title: t('finance_action_col'),
      key: 'action',
      width: 120,
      render: (_: unknown, row: FinanceRecord) => (
        <Button size="small" onClick={() => navigate(`/dashboard?emailId=${encodeURIComponent(row.email_id)}`)}>
          {t('view_email')}
        </Button>
      ),
    },
  ];

  return (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Card bordered={false} title={t('finance_monthly_summary')}>
        <Table
          rowKey={(r: MonthlyFinance) => `${r.month_key}-${r.category}`}
          columns={monthlyColumns as any}
          dataSource={monthly}
          scroll={{ x: 'max-content' }}
          pagination={{ pageSize: 12 }}
        />
      </Card>
      <Card bordered={false} title={emailIdFromQuery ? t('finance_records_filtered', { emailId: emailIdFromQuery }) : t('finance_records')}>
        <Table
          rowKey="id"
          columns={recordColumns as any}
          dataSource={filteredRecords}
          scroll={{ x: 'max-content' }}
          pagination={{ pageSize: 10 }}
          rowClassName={(record: FinanceRecord) => (emailIdFromQuery && record.email_id === emailIdFromQuery ? 'finance-linked-row' : '')}
        />
      </Card>
    </Space>
  );
};

export default FinanceAnalysisPage;
