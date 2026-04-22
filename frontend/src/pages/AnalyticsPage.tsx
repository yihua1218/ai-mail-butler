import React, { useEffect, useState } from 'react';
import { Card, Col, Row, Spin, Statistic, Typography } from 'antd';
import { BarChartOutlined } from '@ant-design/icons';
import axios from 'axios';
import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  PieChart,
  Pie,
  Cell,
  Legend,
} from 'recharts';
import { useAuth } from '../AuthContext';

const { Title } = Typography;

const COLORS = ['#0071e3', '#34c759', '#ff9f0a', '#ff3b30', '#af52de', '#5ac8fa', '#ffcc00'];

interface ChatVolumePoint {
  day: string;
  count: number;
}

interface FinanceSummary {
  month_key: string;
  category: string;
  total_amount: number;
}

interface DashboardStats {
  global_stats?: {
    emails_received: number;
    emails_replied: number;
    registered_users: number;
    ai_replies: number;
  };
  personal_stats?: {
    emails_received: number;
    emails_replied: number;
  };
}

const AnalyticsPage: React.FC = () => {
  const { user } = useAuth();
  const [chatVolume, setChatVolume] = useState<ChatVolumePoint[]>([]);
  const [financeData, setFinanceData] = useState<{ name: string; value: number }[]>([]);
  const [dashStats, setDashStats] = useState<DashboardStats>({});
  const [loadingChat, setLoadingChat] = useState(true);
  const [loadingFinance, setLoadingFinance] = useState(true);
  const [loadingDash, setLoadingDash] = useState(true);

  useEffect(() => {
    const emailParam = user?.email ? `?email=${encodeURIComponent(user.email)}` : '';

    axios.get(`/api/analytics/chat-volume${emailParam}`)
      .then(res => {
        if (res.data?.data) setChatVolume(res.data.data);
      })
      .finally(() => setLoadingChat(false));

    axios.get(`/api/finance/monthly${emailParam}`)
      .then(res => {
        if (res.data?.monthly) {
          const byCategory: Record<string, number> = {};
          (res.data.monthly as FinanceSummary[]).forEach(row => {
            byCategory[row.category] = (byCategory[row.category] || 0) + row.total_amount;
          });
          setFinanceData(Object.entries(byCategory).map(([name, value]) => ({ name, value: Math.abs(value) })));
        }
      })
      .finally(() => setLoadingFinance(false));

    axios.get(`/api/dashboard${emailParam}`)
      .then(res => setDashStats(res.data || {}))
      .finally(() => setLoadingDash(false));
  }, [user]);

  const stats = dashStats.personal_stats || dashStats.global_stats;

  return (
    <div>
      <Title level={4} style={{ marginBottom: 24 }}>
        <BarChartOutlined style={{ marginRight: 8 }} />
        Analytics
      </Title>

      {/* Summary stats */}
      <Row gutter={[16, 16]} style={{ marginBottom: 32 }}>
        <Col xs={12} sm={6}>
          <Card>
            {loadingDash ? <Spin /> : (
              <Statistic title="Emails Received" value={stats?.emails_received ?? 0} />
            )}
          </Card>
        </Col>
        <Col xs={12} sm={6}>
          <Card>
            {loadingDash ? <Spin /> : (
              <Statistic title="Emails Replied" value={stats?.emails_replied ?? 0} />
            )}
          </Card>
        </Col>
        <Col xs={12} sm={6}>
          <Card>
            {loadingDash ? <Spin /> : (
              <Statistic title="AI Replies" value={dashStats.global_stats?.ai_replies ?? 0} />
            )}
          </Card>
        </Col>
        <Col xs={12} sm={6}>
          <Card>
            {loadingChat ? <Spin /> : (
              <Statistic
                title="Chat Messages (30d)"
                value={chatVolume.reduce((s, d) => s + d.count, 0)}
              />
            )}
          </Card>
        </Col>
      </Row>

      <Row gutter={[16, 16]}>
        {/* Chat volume over time */}
        <Col xs={24} lg={14}>
          <Card title="Chat Message Volume (Last 30 Days)">
            {loadingChat ? (
              <div style={{ display: 'flex', justifyContent: 'center', padding: 40 }}><Spin /></div>
            ) : chatVolume.length === 0 ? (
              <Typography.Text type="secondary">No chat data in the last 30 days.</Typography.Text>
            ) : (
              <ResponsiveContainer width="100%" height={260}>
                <BarChart data={chatVolume} margin={{ top: 5, right: 10, left: 0, bottom: 20 }}>
                  <CartesianGrid strokeDasharray="3 3" />
                  <XAxis dataKey="day" angle={-35} textAnchor="end" tick={{ fontSize: 11 }} />
                  <YAxis allowDecimals={false} />
                  <Tooltip />
                  <Bar dataKey="count" fill="#0071e3" name="Messages" radius={[4, 4, 0, 0]} />
                </BarChart>
              </ResponsiveContainer>
            )}
          </Card>
        </Col>

        {/* Finance category breakdown */}
        <Col xs={24} lg={10}>
          <Card title="Finance Category Breakdown">
            {loadingFinance ? (
              <div style={{ display: 'flex', justifyContent: 'center', padding: 40 }}><Spin /></div>
            ) : financeData.length === 0 ? (
              <Typography.Text type="secondary">No finance records found.</Typography.Text>
            ) : (
              <ResponsiveContainer width="100%" height={260}>
                <PieChart>
                  <Pie
                    data={financeData}
                    cx="50%"
                    cy="50%"
                    outerRadius={90}
                    dataKey="value"
                    nameKey="name"
                    label
                    labelLine={false}
                  >
                    {financeData.map((_, index) => (
                      <Cell key={`cell-${index}`} fill={COLORS[index % COLORS.length]} />
                    ))}
                  </Pie>
                  <Legend />
                  <Tooltip />
                </PieChart>
              </ResponsiveContainer>
            )}
          </Card>
        </Col>
      </Row>
    </div>
  );
};

export default AnalyticsPage;
