import React, { useEffect, useMemo, useState } from 'react';
import { Alert, Button, Card, Empty, Select, Segmented, Space, Table, Tag, Typography, type TableColumnsType } from 'antd';
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

type DailyFinanceChartMode = 'expense' | 'both' | 'income';
type ExpensePeriodMode = 'week' | 'month' | 'quarter' | 'year';

const PIE_COLORS = ['#1677ff', '#52c41a', '#fa8c16', '#eb2f96', '#722ed1', '#13c2c2', '#a0d911'];
const FINANCE_CARD_COLLAPSE_KEY = 'ai_mail_butler_finance_collapsed_cards';

const FinanceAnalysisPage: React.FC = () => {
  const { user } = useAuth();
  const { i18n, t } = useTranslation();
  const navigate = useNavigate();
  const location = useLocation();
  const [records, setRecords] = useState<FinanceRecord[]>([]);
  const [monthly, setMonthly] = useState<MonthlyFinance[]>([]);
  const [dailyChartMode, setDailyChartMode] = useState<DailyFinanceChartMode>('expense');
  const [expensePeriodMode, setExpensePeriodMode] = useState<ExpensePeriodMode>('week');
  const [selectedMonthKey, setSelectedMonthKey] = useState<string>('');
  const [selectedDailyKey, setSelectedDailyKey] = useState<string>('');
  const [recordPagination, setRecordPagination] = useState({ current: 1, pageSize: 10 });
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

  const dayKeyFormatter = useMemo(() => new Intl.DateTimeFormat('en-CA', {
    timeZone: user?.timezone || 'UTC',
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
  }), [user?.timezone]);

  const getDayKey = (date: Date) => {
    const parts = dayKeyFormatter.formatToParts(date);
    const year = parts.find((part) => part.type === 'year')?.value || '0000';
    const month = parts.find((part) => part.type === 'month')?.value || '01';
    const day = parts.find((part) => part.type === 'day')?.value || '01';
    return `${year}-${month}-${day}`;
  };

  const getRecordDayKey = (row: FinanceRecord) => {
    const iso = row.created_at.includes('T') ? row.created_at : `${row.created_at.replace(' ', 'T')}Z`;
    const date = new Date(iso);
    return Number.isNaN(date.getTime()) ? '' : getDayKey(date);
  };

  const getFinanceMonthKey = (row: FinanceRecord) => row.transaction_month_key || row.month_key;

  const isExpenseRecord = (row: FinanceRecord) => (
    row.direction === 'expense' || row.category === 'expense' || row.finance_type === 'bill'
  );

  const isIncomeRecord = (row: FinanceRecord) => (
    row.direction === 'income' || row.direction === 'deposit' || row.category === 'income' || row.category === 'deposit'
  );

  const formatDayKey = (dayKey: string) => {
    if (!dayKey) return '';
    const [year, month, day] = dayKey.split('-');
    const date = new Date(`${year}-${month}-${day}T00:00:00Z`);
    if (Number.isNaN(date.getTime())) return dayKey;
    return new Intl.DateTimeFormat(i18n.language === 'zh-TW' ? 'zh-TW' : 'en-US', {
      timeZone: 'UTC',
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
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
    axios.get(`/api/finance/records?email=${encodeURIComponent(user.email)}&limit=10000`).then((res) => {
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

  const clearDailyFilter = () => setSelectedDailyKey('');

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

  const availableMonthKeys = useMemo(() => {
    const keys = new Set<string>();
    monthly.forEach((row) => {
      if (/^\d{4}-\d{2}$/.test(row.month_key)) keys.add(row.month_key);
    });
    records.forEach((row) => {
      const key = getFinanceMonthKey(row);
      if (/^\d{4}-\d{2}$/.test(key)) keys.add(key);
    });
    keys.add(currentMonthKey);
    return Array.from(keys).sort((a, b) => b.localeCompare(a));
  }, [monthly, records, currentMonthKey]);

  useEffect(() => {
    setSelectedMonthKey((current) => current || currentMonthKey);
  }, [currentMonthKey]);

  const selectedIncomeExpenseMonthKey = selectedMonthKey || currentMonthKey;

  const incomeExpensePieData = useMemo(() => {
    const totals = new Map<string, number>([
      ['income', 0],
      ['expense', 0],
    ]);
    records
      .filter((row) => getFinanceMonthKey(row) === selectedIncomeExpenseMonthKey)
      .forEach((row) => {
        const amount = Math.abs(Number(row.amount) || 0);
        if (isIncomeRecord(row)) {
          totals.set('income', (totals.get('income') || 0) + amount);
        } else if (isExpenseRecord(row)) {
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
  }, [records, selectedIncomeExpenseMonthKey]);

  const incomeExpensePieTotal = incomeExpensePieData.reduce((sum, item) => sum + item.value, 0);
  const pieBackground = incomeExpensePieData.reduce<{ cursor: number; segments: string[] }>((acc, item) => {
    const start = acc.cursor;
    const end = start + (item.value / incomeExpensePieTotal) * 100;
    acc.segments.push(`${item.color} ${start}% ${end}%`);
    acc.cursor = end;
    return acc;
  }, { cursor: 0, segments: [] }).segments.join(', ');

  const dailyFinanceLast30Days = useMemo(() => {
    const timezone = user?.timezone || 'UTC';
    const dayLabelFormatter = new Intl.DateTimeFormat(i18n.language === 'zh-TW' ? 'zh-TW' : 'en-US', {
      timeZone: timezone,
      month: '2-digit',
      day: '2-digit',
    });

    const dayKeys = Array.from({ length: 30 }, (_, index) => {
      const date = new Date();
      date.setDate(date.getDate() - (29 - index));
      return getDayKey(date);
    });

    const totals = new Map<string, { income: number; expense: number }>();
    dayKeys.forEach((key) => totals.set(key, { income: 0, expense: 0 }));

    records.forEach((row) => {
      const isExpense = isExpenseRecord(row);
      const isIncome = isIncomeRecord(row);
      if (!isExpense && !isIncome) return;
      const amount = Math.abs(Number(row.amount) || 0);
      if (!amount) return;
      const key = getRecordDayKey(row);
      if (!totals.has(key)) return;
      const current = totals.get(key) || { income: 0, expense: 0 };
      totals.set(key, {
        income: current.income + (isIncome ? amount : 0),
        expense: current.expense + (isExpense ? amount : 0),
      });
    });

    const bars = dayKeys.map((key) => {
      const [year, month, day] = key.split('-');
      const labelDate = new Date(`${year}-${month}-${day}T00:00:00Z`);
      return {
        key,
        label: Number.isNaN(labelDate.getTime()) ? `${month}/${day}` : dayLabelFormatter.format(labelDate),
        income: totals.get(key)?.income || 0,
        expense: totals.get(key)?.expense || 0,
      };
    });

    const visibleValues = bars.flatMap((item) => {
      if (dailyChartMode === 'income') return [item.income];
      if (dailyChartMode === 'both') return [item.income, item.expense];
      return [item.expense];
    });
    const maxValue = visibleValues.reduce((max, value) => Math.max(max, value), 0);
    const niceMaxValue = (() => {
      if (maxValue <= 0) return 0;
      const exponent = Math.floor(Math.log10(maxValue));
      const magnitude = 10 ** exponent;
      const normalized = maxValue / magnitude;
      const niceNormalized = normalized <= 1 ? 1 : normalized <= 2 ? 2 : normalized <= 5 ? 5 : 10;
      return niceNormalized * magnitude;
    })();
    const yAxisTicks = Array.from({ length: 5 }, (_, index) => Math.round(niceMaxValue * ((4 - index) / 4)));
    return {
      bars,
      maxValue: niceMaxValue,
      yAxisTicks,
    };
  }, [records, user?.timezone, i18n.language, dailyChartMode, dayKeyFormatter]);

  const dailyChartSeries = useMemo(() => {
    if (dailyChartMode === 'income') {
      return [{ key: 'income', label: t('finance_chart_income'), className: 'income' }];
    }
    if (dailyChartMode === 'both') {
      return [
        { key: 'expense', label: t('finance_chart_expense'), className: 'expense' },
        { key: 'income', label: t('finance_chart_income'), className: 'income' },
      ];
    }
    return [{ key: 'expense', label: t('finance_chart_expense'), className: 'expense' }];
  }, [dailyChartMode, t]);

  const expensePeriodSummary = useMemo(() => {
    const timezone = user?.timezone || 'UTC';
    const dayFormatter = new Intl.DateTimeFormat('en-CA', {
      timeZone: timezone,
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
    });
    const labelLocale = i18n.language === 'zh-TW' ? 'zh-TW' : 'en-US';
    const displayMonthFormatter = new Intl.DateTimeFormat(labelLocale, {
      timeZone: 'UTC',
      year: 'numeric',
      month: 'short',
    });

    const weekStartKey = (date: Date) => {
      const localDayKey = dayFormatter.format(date);
      const [year, month, day] = localDayKey.split('-').map(Number);
      const localDate = new Date(Date.UTC(year, month - 1, day));
      const dayOfWeek = localDate.getUTCDay();
      const diffToMonday = (dayOfWeek + 6) % 7;
      localDate.setUTCDate(localDate.getUTCDate() - diffToMonday);
      return localDate.toISOString().slice(0, 10);
    };

    const periodRows = new Map<string, { key: string; label: string; amount: number }>();
    records.forEach((row) => {
      if (!isExpenseRecord(row)) return;
      const amount = Math.abs(Number(row.amount) || 0);
      if (!amount) return;

      let key = '';
      let label = '';
      if (expensePeriodMode === 'week') {
        const iso = row.created_at.includes('T') ? row.created_at : `${row.created_at.replace(' ', 'T')}Z`;
        const date = new Date(iso);
        if (Number.isNaN(date.getTime())) return;
        key = weekStartKey(date);
        label = t('finance_period_week_label', { date: formatDayKey(key) });
      } else {
        const monthKey = getFinanceMonthKey(row);
        if (!/^\d{4}-\d{2}$/.test(monthKey)) return;
        const [year, month] = monthKey.split('-').map(Number);
        if (expensePeriodMode === 'month') {
          key = monthKey;
          label = displayMonthFormatter.format(new Date(Date.UTC(year, month - 1, 1)));
        } else if (expensePeriodMode === 'quarter') {
          const quarter = Math.floor((month - 1) / 3) + 1;
          key = `${year}-Q${quarter}`;
          label = t('finance_period_quarter_label', { year, quarter });
        } else {
          key = String(year);
          label = String(year);
        }
      }

      const current = periodRows.get(key) || { key, label, amount: 0 };
      periodRows.set(key, { ...current, amount: current.amount + amount });
    });

    const bars = Array.from(periodRows.values()).sort((a, b) => a.key.localeCompare(b.key)).slice(-12);
    const maxValue = bars.reduce((max, item) => Math.max(max, item.amount), 0);
    const niceMaxValue = (() => {
      if (maxValue <= 0) return 0;
      const exponent = Math.floor(Math.log10(maxValue));
      const magnitude = 10 ** exponent;
      const normalized = maxValue / magnitude;
      const niceNormalized = normalized <= 1 ? 1 : normalized <= 2 ? 2 : normalized <= 5 ? 5 : 10;
      return niceNormalized * magnitude;
    })();
    const yAxisTicks = Array.from({ length: 5 }, (_, index) => Math.round(niceMaxValue * ((4 - index) / 4)));
    return { bars, maxValue: niceMaxValue, yAxisTicks };
  }, [records, user?.timezone, i18n.language, expensePeriodMode, dayKeyFormatter, t]);

  useEffect(() => {
    setRecordPagination((prev) => ({ ...prev, current: 1 }));
  }, [emailIdFromQuery, selectedDailyKey]);

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

  const emailFilteredRecords = emailIdFromQuery
    ? records.filter((row) => row.email_id === emailIdFromQuery)
    : records;
  const filteredRecords = selectedDailyKey
    ? emailFilteredRecords.filter((row) => getRecordDayKey(row) === selectedDailyKey)
    : emailFilteredRecords;

  const recordTablePagination = {
    current: recordPagination.current,
    pageSize: recordPagination.pageSize,
    showSizeChanger: true,
    pageSizeOptions: [5, 10, 20, 50, 100],
    showTotal: (total: number, range: [number, number]) => t('pagination_total', { from: range[0], to: range[1], total }),
    onChange: (page: number, pageSize: number) => {
      setRecordPagination({ current: page, pageSize });
    },
  };

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
      <CollapsibleCard
        storageKey="income-expense-ratio"
        title={t('finance_income_expense_pie')}
        extra={
          <Select
            size="small"
            value={selectedIncomeExpenseMonthKey}
            style={{ minWidth: 120 }}
            onChange={setSelectedMonthKey}
            options={availableMonthKeys.map((key) => ({ label: key, value: key }))}
          />
        }
      >
        {incomeExpensePieData.length > 0 ? (
          <div className="finance-pie-layout">
            <div className="finance-pie-chart" style={{ background: `conic-gradient(${pieBackground})` }}>
              <div className="finance-pie-center">
                <Text type="secondary">{selectedIncomeExpenseMonthKey}</Text>
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
      <CollapsibleCard
        storageKey="expense-period-summary"
        title={t('finance_expense_period_summary')}
        extra={
          <Segmented
            size="small"
            value={expensePeriodMode}
            onChange={(value) => setExpensePeriodMode(value as ExpensePeriodMode)}
            options={[
              { label: t('finance_period_week'), value: 'week' },
              { label: t('finance_period_month'), value: 'month' },
              { label: t('finance_period_quarter'), value: 'quarter' },
              { label: t('finance_period_year'), value: 'year' },
            ]}
          />
        }
      >
        {expensePeriodSummary.maxValue > 0 ? (
          <div className="finance-bar-chart-scroll">
            <div className="finance-bar-chart">
              <div className="finance-bar-y-axis" aria-hidden="true">
                {expensePeriodSummary.yAxisTicks.map((tick, index) => (
                  <span key={`${tick}-${index}`}>{tick.toLocaleString()}</span>
                ))}
              </div>
              <div className="finance-bar-plot">
                <div className="finance-bar-grid" aria-hidden="true">
                  {expensePeriodSummary.yAxisTicks.map((tick, index) => (
                    <span key={`${tick}-${index}`} />
                  ))}
                </div>
                <div className="finance-bar-chart-layout finance-period-chart-layout" style={{ '--finance-period-columns': expensePeriodSummary.bars.length } as React.CSSProperties}>
                  {expensePeriodSummary.bars.map((item) => {
                    const heightPercent = expensePeriodSummary.maxValue > 0
                      ? Math.max(4, Math.round((item.amount / expensePeriodSummary.maxValue) * 100))
                      : 0;
                    const tooltip = `${item.label}: ${item.amount.toLocaleString()}`;
                    return (
                      <div className="finance-bar-column" key={item.key} title={tooltip}>
                        <div className="finance-bar-series is-single">
                          <div className="finance-bar-track" data-tooltip={tooltip} aria-label={tooltip}>
                            <div className="finance-bar-fill expense" style={{ height: `${heightPercent}%` }} />
                          </div>
                        </div>
                        <span className="finance-bar-label">{item.label}</span>
                      </div>
                    );
                  })}
                </div>
              </div>
            </div>
          </div>
        ) : (
          <Empty description={t('finance_no_expense_period_data')} />
        )}
      </CollapsibleCard>
      <CollapsibleCard
        storageKey="daily-expense-30-days"
        title={t('finance_daily_amount_30d')}
        extra={
          <Segmented
            size="small"
            value={dailyChartMode}
            onChange={(value) => setDailyChartMode(value as DailyFinanceChartMode)}
            options={[
              { label: t('finance_chart_expense'), value: 'expense' },
              { label: t('finance_chart_income_expense'), value: 'both' },
              { label: t('finance_chart_income'), value: 'income' },
            ]}
          />
        }
      >
        {dailyFinanceLast30Days.maxValue > 0 ? (
          <div className="finance-bar-chart-scroll">
            <div className="finance-bar-chart">
              <div className="finance-bar-y-axis" aria-hidden="true">
                {dailyFinanceLast30Days.yAxisTicks.map((tick, index) => (
                  <span key={`${tick}-${index}`}>{tick.toLocaleString()}</span>
                ))}
              </div>
              <div className="finance-bar-plot">
                <div className="finance-bar-grid" aria-hidden="true">
                  {dailyFinanceLast30Days.yAxisTicks.map((tick, index) => (
                    <span key={`${tick}-${index}`} />
                  ))}
                </div>
                <div className="finance-bar-chart-layout">
                  {dailyFinanceLast30Days.bars.map((item, index) => (
                    <div
                      className={`finance-bar-column ${selectedDailyKey === item.key ? 'is-selected' : ''}`}
                      key={item.key}
                      role="button"
                      tabIndex={0}
                      aria-pressed={selectedDailyKey === item.key}
                      onClick={() => setSelectedDailyKey((current) => current === item.key ? '' : item.key)}
                      onKeyDown={(event) => {
                        if (event.key === 'Enter' || event.key === ' ') {
                          event.preventDefault();
                          setSelectedDailyKey((current) => current === item.key ? '' : item.key);
                        }
                      }}
                    >
                      <div className={`finance-bar-series ${dailyChartMode === 'both' ? 'is-grouped' : 'is-single'}`}>
                        {dailyChartSeries.map((series) => {
                          const value = series.key === 'income' ? item.income : item.expense;
                          const heightPercent = dailyFinanceLast30Days.maxValue > 0
                            ? Math.max(4, Math.round((value / dailyFinanceLast30Days.maxValue) * 100))
                            : 0;
                          const tooltip = `${item.label} ${series.label}: ${value.toLocaleString()}`;
                          return (
                            <div
                              className="finance-bar-track"
                              title={tooltip}
                              data-tooltip={tooltip}
                              aria-label={tooltip}
                              key={series.key}
                            >
                              <div className={`finance-bar-fill ${series.className}`} style={{ height: `${heightPercent}%` }} />
                            </div>
                          );
                        })}
                      </div>
                      <span className="finance-bar-label">{index % 5 === 0 || index === dailyFinanceLast30Days.bars.length - 1 ? item.label : ''}</span>
                    </div>
                  ))}
                </div>
                {dailyChartMode === 'both' && (
                  <div className="finance-bar-legend">
                    {dailyChartSeries.map((series) => (
                      <span key={series.key}>
                        <i className={`finance-bar-legend-swatch ${series.className}`} />
                        {series.label}
                      </span>
                    ))}
                  </div>
                )}
              </div>
            </div>
          </div>
        ) : (
          <Empty description={t('finance_no_daily_amount_30d')} />
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
            {selectedDailyKey && (
              <Text type="secondary">
                {t('finance_filtering_day')}: {formatDayKey(selectedDailyKey)}
              </Text>
            )}
            {isFilteringByEmail && <Button size="small" onClick={clearEmailFilter}>{t('finance_clear_filter')}</Button>}
            {selectedDailyKey && <Button size="small" onClick={clearDailyFilter}>{t('finance_clear_daily_filter')}</Button>}
            <Button size="small" onClick={backToDashboard}>{t('finance_back_dashboard')}</Button>
          </Space>
        ) : selectedDailyKey ? (
          <Space wrap>
            <Text type="secondary">
              {t('finance_filtering_day')}: {formatDayKey(selectedDailyKey)}
            </Text>
            <Button size="small" onClick={clearDailyFilter}>{t('finance_clear_daily_filter')}</Button>
          </Space>
        ) : null}
      >
        <Table
          rowKey="id"
          columns={recordColumns}
          dataSource={filteredRecords}
          scroll={{ x: 'max-content' }}
          pagination={recordTablePagination}
          rowClassName={(record: FinanceRecord) => (linkedEmailId && record.email_id === linkedEmailId ? 'finance-linked-row' : '')}
        />
      </CollapsibleCard>
    </Space>
  );
};

export default FinanceAnalysisPage;
