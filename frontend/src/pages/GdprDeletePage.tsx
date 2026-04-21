import React, { useEffect, useState } from 'react';
import { Alert, Button, Card, Checkbox, Table, Typography, message } from 'antd';
import axios from 'axios';
import { useLocation } from 'react-router-dom';

const { Paragraph } = Typography;

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

export default GdprDeletePage;
