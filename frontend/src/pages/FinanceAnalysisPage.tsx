import React, { useEffect, useState } from 'react';
import { Alert, Card, Space, Table, Tag } from 'antd';
import axios from 'axios';
import { useTranslation } from 'react-i18next';
import { useAuth } from '../AuthContext';

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

export default FinanceAnalysisPage;
