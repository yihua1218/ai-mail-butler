import React, { useEffect, useMemo, useState } from 'react';
import { Alert, Button, Card, Empty, Space, Table, Tag, Typography, type TableColumnsType } from 'antd';
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

const PIE_COLORS = ['#1677ff', '#52c41a', '#fa8c16', '#eb2f96', '#722ed1', '#13c2c2', '#a0d911'];
const FINANCE_CARD_COLLAPSE_KEY = 'ai_mail_butler_finance_collapsed_cards';

const FinanceAnalysisPage: React.FC = () => {
  const { user } = useAuth();
  const { i18n, t } = useTranslation();
  const navigate = useNavigate();
  const location = useLocation();
  const [records, setRecords] = useState<FinanceRecord[]>([]);
  const [monthly, setMonthly] = useState<MonthlyFinance[]>([]);
  const [collapsedCards, setCollapsedCards] = useState<Record<string, boolean>>(() => {
    try {
      return JSON.parse(localStorage.getItem(FINANCE_CARD_COLLAPSE_KEY) || '{}');
    } catch {
      return {};
    }
  });
  const { Text } = Typography;

  const toggleCard = (key: string) => {
    setCollapsedCards((prev) => {
      const next = { ...prev, [key]: !prev[key] };
      localStorage.setItem(FINANCE_CARD_COLLAPSE_KEY, JSON.stringify(next));
      return next;
    });
  };

  const CollapsibleCard = ({
    storageKey,
    title,
    extra,
    children,
  }: {
    storageKey: string;
    title: React.ReactNode;
    extra?: React.ReactNode;
    children: React.ReactNode;
  }) => {
    const collapsed = !!collapsedCards[storageKey];
    return (
      <Card
        bordered={false}
        title={title}
        extra={
          <Space>
            {extra}
            <Button size="small" onClick={() => toggleCard(storageKey)}>
              {collapsed ? t('expand') : t('collapse')}
            </Button>
          </Space>
        }
      >
        {!collapsed && children}
      </Card>
    );
  };

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

  const highlightedEmailIdFromQuery = useMemo(() => {
    const params = new URLSearchParams(location.search);
    return params.get('highlightEmailId') || '';
  }, [location.search]);

  const subjectFromQuery = useMemo(() => {
    const params = new URLSearchParams(location.search);
    return params.get('subject') || '';
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

  const monthlyCategoryColor = (category: string) => {
    if (category === 'deposit' || category === 'income') return 'green';
    if (category === 'expense') return 'volcano';
    return 'blue';
  };

  const linkedEmailId = emailIdFromQuery || highlightedEmailIdFromQuery;
  const linkedRecord = linkedEmailId ? records.find((row) => row.email_id === linkedEmailId) : undefined;
  const linkedSubject = linkedRecord?.subject || subjectFromQuery || linkedEmailId;
  const isFilteringByEmail = Boolean(emailIdFromQuery);

  const buildFinanceLink = (params: Record<string, string>) => {
    const next = new URLSearchParams();
    Object.entries(params).forEach(([key, value]) => {
      if (value) next.set(key, value);
    });
    const query = next.toString();
    return query ? `/finance?${query}` : '/finance';
  };

  const clearEmailFilter = () => {
    if (!linkedEmailId) {
      navigate('/finance');
      return;
    }
    navigate(buildFinanceLink({ highlightEmailId: linkedEmailId, subject: linkedSubject }));
  };

  const backToDashboard = () => {
    navigate(linkedEmailId ? `/dashboard?emailId=${encodeURIComponent(linkedEmailId)}` : '/dashboard');
  };

  const currentMonthKey = useMemo(() => {
    const timezone = user?.timezone || 'UTC';
    const parts = new Intl.DateTimeFormat('en-CA', {
      timeZone: timezone,
      year: 'numeric',
      month: '2-digit',
    }).formatToParts(new Date());
    const year = parts.find((part) => part.type === 'year')?.value || new Date().getFullYear().toString();
    const month = parts.find((part) => part.type === 'month')?.value || String(new Date().getMonth() + 1).padStart(2, '0');
    return `${year}-${month}`;
  }, [user?.timezone]);

  const incomeExpensePieData = useMemo(() => {
    const totals = new Map<string, number>([
      ['income', 0],
      ['expense', 0],
    ]);
    records
      .filter((row) => (row.transaction_month_key || row.month_key) === currentMonthKey)
      .forEach((row) => {
        const amount = Math.abs(Number(row.amount) || 0);
        if (row.direction === 'income' || row.direction === 'deposit' || row.category === 'income' || row.category === 'deposit') {
          totals.set('income', (totals.get('income') || 0) + amount);
        } else if (row.direction === 'expense' || row.category === 'expense' || row.finance_type === 'bill') {
          totals.set('expense', (totals.get('expense') || 0) + amount);
        }
      });
    const colors: Record<string, string> = {
      income: '#2f9e44',
      expense: '#e8590c',
    };
    return Array.from(totals.entries())
      .map(([category, value]) => ({ category, value, color: colors[category] || PIE_COLORS[0] }))
      .filter((item) => item.value > 0)
      .sort((a, b) => b.value - a.value);
  }, [records, currentMonthKey]);

  const incomeExpensePieTotal = incomeExpensePieData.reduce((sum, item) => sum + item.value, 0);
  const pieBackground = incomeExpensePieData.reduce<{ cursor: number; segments: string[] }>((acc, item) => {
    const start = acc.cursor;
    const end = start + (item.value / incomeExpensePieTotal) * 100;
    acc.segments.push(`${item.color} ${start}% ${end}%`);
    acc.cursor = end;
    return acc;
  }, { cursor: 0, segments: [] }).segments.join(', ');

  if (!user) {
    return (
      <Card bordered={false}>
        <Alert type="info" showIcon message={t('finance_login_required')} description={t('finance_login_desc')} />
      </Card>
    );
  }

  const monthlyColumns: TableColumnsType<MonthlyFinance> = [
    { title: t('finance_month_col'), dataIndex: 'month_key', key: 'month_key', width: 120 },
    { title: t('finance_category_col'), dataIndex: 'category', key: 'category', width: 120, render: (v: string) => <Tag color={monthlyCategoryColor(v)}>{t(`finance_cat_${v}`, { defaultValue: v })}</Tag> },
    { title: t('finance_total_amount_col'), dataIndex: 'total_amount', key: 'total_amount', width: 180, render: (v: number) => v?.toLocaleString() ?? '0' },
    { title: t('finance_updated_at_col'), dataIndex: 'updated_at', key: 'updated_at', width: 200, render: (v: string) => <span style={{ whiteSpace: 'nowrap' }}>{formatInUserTimezone(v)}</span> },
  ];

  const filteredRecords = emailIdFromQuery
    ? records.filter((row) => row.email_id === emailIdFromQuery)
    : records;

  const recordColumns: TableColumnsType<FinanceRecord> = [
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
      <CollapsibleCard storageKey="income-expense-ratio" title={t('finance_income_expense_pie')}>
        {incomeExpensePieData.length > 0 ? (
          <div className="finance-pie-layout">
            <div className="finance-pie-chart" style={{ background: `conic-gradient(${pieBackground})` }}>
              <div className="finance-pie-center">
                <Text type="secondary">{currentMonthKey}</Text>
                <strong>{incomeExpensePieTotal.toLocaleString()}</strong>
              </div>
            </div>
            <div className="finance-pie-legend">
              {incomeExpensePieData.map((item) => (
                <div className="finance-pie-legend-row" key={item.category}>
                  <span className="finance-pie-swatch" style={{ background: item.color }} />
                  <span>{t(`finance_cat_${item.category}`, { defaultValue: item.category })}</span>
                  <Text type="secondary">
                    {item.value.toLocaleString()} ({Math.round((item.value / incomeExpensePieTotal) * 100)}%)
                  </Text>
                </div>
              ))}
            </div>
          </div>
        ) : (
          <Empty description={t('finance_no_income_expense_mix')} />
        )}
      </CollapsibleCard>
      <CollapsibleCard storageKey="monthly-summary" title={t('finance_monthly_summary')}>
        <Table
          rowKey={(r: MonthlyFinance) => `${r.month_key}-${r.category}`}
          columns={monthlyColumns}
          dataSource={monthly}
          scroll={{ x: 'max-content' }}
          pagination={{ pageSize: 12 }}
        />
      </CollapsibleCard>
      <CollapsibleCard
        storageKey="records"
        title={isFilteringByEmail ? t('finance_records_filtered', { subject: linkedSubject }) : t('finance_records')}
        extra={linkedEmailId ? (
          <Space wrap>
            <Text type="secondary">
              {isFilteringByEmail ? t('finance_filtering_by') : t('finance_highlighting')}: {linkedSubject}
            </Text>
            {isFilteringByEmail && <Button size="small" onClick={clearEmailFilter}>{t('finance_clear_filter')}</Button>}
            <Button size="small" onClick={backToDashboard}>{t('finance_back_dashboard')}</Button>
          </Space>
        ) : null}
      >
        <Table
          rowKey="id"
          columns={recordColumns}
          dataSource={filteredRecords}
          scroll={{ x: 'max-content' }}
          pagination={{ pageSize: 10 }}
          rowClassName={(record: FinanceRecord) => (linkedEmailId && record.email_id === linkedEmailId ? 'finance-linked-row' : '')}
        />
      </CollapsibleCard>
    </Space>
  );
};

export default FinanceAnalysisPage;
