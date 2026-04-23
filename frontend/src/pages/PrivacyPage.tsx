import React, { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useAuth } from '../AuthContext';
import {
  Card,
  Typography,
  Button,
  Switch,
  Select,
  Table,
  Tag,
  message,
  Divider,
  List,
  Alert,
  Space,
  Statistic,
  Row,
  Col,
  Tabs,
  Checkbox,
  Input,
} from 'antd';
import {
  SafetyCertificateOutlined,
  SafetyOutlined,
  DeleteOutlined,
  ExportOutlined,
  FileTextOutlined,
  EuroCircleOutlined,
} from '@ant-design/icons';

const { Title, Text } = Typography;
const { Option } = Select;

interface ConsentRecord {
  id: string;
  policy_version: string;
  consent_type: string;
  consent_granted: boolean;
  consent_source: string;
  created_at: string;
}

interface DsarRequest {
  id: string;
  request_type: string;
  status: string;
  completed_at: string | null;
  created_at: string;
}

interface PrivacySettings {
  do_not_sell_share: boolean;
  cross_border_disclosure_given: boolean;
  data_location_preference: string | null;
  updated_at: string;
}

interface AgeVerification {
  is_minor: boolean;
  guardian_consent_given: boolean;
  guardian_email: string | null;
  age_verified_at: string | null;
}

interface RetentionPolicy {
  id: string;
  data_type: string;
  retention_days: number;
  is_active: boolean;
}

const PrivacyPage: React.FC = () => {
  const { t } = useTranslation();
  const { user, api } = useAuth();
  const [consentHistory, setConsentHistory] = useState<ConsentRecord[]>([]);
  const [dsarRequests, setDsarRequests] = useState<DsarRequest[]>([]);
  const [privacySettings, setPrivacySettings] = useState<PrivacySettings | null>(null);
  const [ageVerification, setAgeVerification] = useState<AgeVerification | null>(null);
  const [retentionPolicies, setRetentionPolicies] = useState<RetentionPolicy[]>([]);

  const fetchData = async () => {
    if (!user) return;
    try {
      const [consentRes, dsarRes, privacyRes, ageRes, retentionRes] = await Promise.all([
        fetch(`${api}/consent/history?email=${user.email}`).then(r => r.json()),
        fetch(`${api}/dsar/status?email=${user.email}`).then(r => r.json()),
        fetch(`${api}/privacy/settings?email=${user.email}`).then(r => r.json()),
        fetch(`${api}/privacy/age-verification?email=${user.email}`).then(r => r.json()),
        fetch(`${api}/admin/retention/policies`).then(r => r.json()),
      ]);

      if (consentRes.status === 'success') setConsentHistory(consentRes.history || []);
      if (dsarRes.status === 'success') setDsarRequests(dsarRes.requests || []);
      if (privacyRes.status === 'success') setPrivacySettings(privacyRes.settings || null);
      if (ageRes.status === 'success') setAgeVerification(ageRes.verification || null);
      if (retentionRes.status === 'success') setRetentionPolicies(retentionRes.policies || []);
    } catch (err) {
      console.error('Failed to fetch privacy data:', err);
    }
  };

  useEffect(() => {
    fetchData();
  }, [user, api]);

  const handleConsentUpdate = async (consentType: string, granted: boolean) => {
    try {
      const res = await fetch(`${api}/consent/update`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ email: user?.email, consent_type: consentType, consent_granted: granted }),
      });
      const data = await res.json();
      if (data.status === 'success') {
        message.success(t('privacy.consent_updated'));
        fetchData();
      } else {
        message.error(data.message === 'User not found' ? t('privacy.user_not_found') : (data.message || t('common.error')));
      }
    } catch {
      message.error(t('common.error'));
    }
  };

  const handleDsarRequest = async (requestType: string) => {
    try {
      const res = await fetch(`${api}/dsar/request`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ email: user?.email, request_type: requestType }),
      });
      const data = await res.json();
      if (data.status === 'success') {
        message.success(t('privacy.dsar_requested'));
        fetchData();
      } else {
        message.error(data.message === 'User not found' ? t('privacy.user_not_found') : (data.message || t('common.error')));
      }
    } catch {
      message.error(t('common.error'));
    }
  };

  const handlePrivacySettings = async (settings: Partial<PrivacySettings>) => {
    try {
      const res = await fetch(`${api}/privacy/settings`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          email: user?.email,
          do_not_sell_share: settings.do_not_sell_share,
          cross_border_disclosure_given: settings.cross_border_disclosure_given,
          data_location_preference: settings.data_location_preference,
        }),
      });
      const data = await res.json();
      if (data.status === 'success') {
        message.success(t('privacy.settings_updated'));
        fetchData();
      } else {
        message.error(data.message === 'User not found' ? t('privacy.user_not_found') : (data.message || t('common.error')));
      }
    } catch {
      message.error(t('common.error'));
    }
  };

  const handleAgeVerification = async (isMinor: boolean, guardianEmail?: string) => {
    try {
      const res = await fetch(`${api}/privacy/age-verification`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          email: user?.email,
          is_minor: isMinor,
          guardian_consent_given: isMinor,
          guardian_email: guardianEmail,
        }),
      });
      const data = await res.json();
      if (data.status === 'success') {
        message.success(t('privacy.age_verified'));
        fetchData();
      } else {
        message.error(data.message === 'User not found' ? t('privacy.user_not_found') : (data.message || t('common.error')));
      }
    } catch {
      message.error(t('common.error'));
    }
  };

  const handleRunPurge = async () => {
    try {
      const res = await fetch(`${api}/admin/retention/purge`, { method: 'POST' });
      const data = await res.json();
      if (data.status === 'success') {
        message.success(`Purged: ${data.purged?.join(', ') || 'none'}`);
        fetchData();
      }
    } catch {
      message.error(t('common.error'));
    }
  };

  const consentColumns = [
    { title: t('privacy.consent_type'), dataIndex: 'consent_type', key: 'consent_type' },
    { title: t('privacy.granted'), dataIndex: 'consent_granted', key: 'consent_granted', render: (v: boolean) => <Tag color={v ? 'green' : 'red'}>{v ? 'Yes' : 'No'}</Tag> },
    { title: t('privacy.source'), dataIndex: 'consent_source', key: 'consent_source' },
    { title: t('privacy.date'), dataIndex: 'created_at', key: 'created_at' },
  ];

  const dsarColumns = [
    { title: t('privacy.request_type'), dataIndex: 'request_type', key: 'request_type', render: (v: string) => <Tag>{v}</Tag> },
    { title: t('privacy.status'), dataIndex: 'status', key: 'status', render: (v: string) => <Tag color={v === 'completed' ? 'green' : 'orange'}>{v}</Tag> },
    { title: t('privacy.date'), dataIndex: 'created_at', key: 'created_at' },
    { title: t('privacy.completed'), dataIndex: 'completed_at', key: 'completed_at', render: (v: string) => v || '-' },
  ];

  const retentionColumns = [
    { title: t('privacy.data_type'), dataIndex: 'data_type', key: 'data_type' },
    { title: t('privacy.retention_days'), dataIndex: 'retention_days', key: 'retention_days' },
    { title: t('privacy.active'), dataIndex: 'is_active', key: 'is_active', render: (v: boolean) => <Tag color={v ? 'green' : 'default'}>{v ? 'Active' : 'Inactive'}</Tag> },
  ];

  return (
    <div style={{ maxWidth: 1200, margin: '0 auto' }}>
      <Space direction="vertical" size="large" style={{ width: '100%' }}>
        <Title level={2}>
          <SafetyCertificateOutlined /> {t('privacy.title')}
        </Title>

        <Tabs
          defaultActiveKey="overview"
          items={[
            {
              key: 'overview',
              label: t('privacy.overview'),
              children: (
                <Row gutter={[16, 16]}>
                  <Col xs={24} md={8}>
                    <Card>
                      <Statistic
                        title={t('privacy.consent_records')}
                        value={consentHistory.length}
                        prefix={<SafetyOutlined />}
                      />
                    </Card>
                  </Col>
                  <Col xs={24} md={8}>
                    <Card>
                      <Statistic
                        title={t('privacy.dsar_requests')}
                        value={dsarRequests.length}
                        prefix={<FileTextOutlined />}
                      />
                    </Card>
                  </Col>
                  <Col xs={24} md={8}>
                    <Card>
                      <Statistic
                        title={t('privacy.do_not_sell')}
                        value={privacySettings?.do_not_sell_share ? 1 : 0}
                        suffix="/ 1"
                        prefix={<SafetyCertificateOutlined />}
                      />
                    </Card>
                  </Col>
                </Row>
              ),
            },
            {
              key: 'consent',
              label: t('privacy.consent'),
              children: (
                <Card title={t('privacy.consent_title')}>
                  <Alert
                    message={t('privacy.consent_info')}
                    type="info"
                    showIcon
                    style={{ marginBottom: 16 }}
                  />
                  <Space direction="vertical" style={{ width: '100%', marginBottom: 16 }}>
                    <Switch
                      checked={user?.training_data_consent || false}
                      onChange={(checked) => handleConsentUpdate('training_data', checked)}
                      checkedChildren="Yes"
                      unCheckedChildren="No"
                    />
                    <Text type="secondary">{t('training_data_consent')}</Text>
                  </Space>
                  <Divider>{t('privacy.consent_records')}</Divider>
                  <Table
                    dataSource={consentHistory}
                    columns={consentColumns}
                    rowKey="id"
                    pagination={{ pageSize: 5 }}
                    locale={{ emptyText: t('privacy.no_consent_records') }}
                  />
                </Card>
              ),
            },
            {
              key: 'rights',
              label: t('privacy.data_rights'),
              children: (
                <Card title={t('privacy.dsar_title')}>
                  <Alert
                    message={t('privacy.dsar_info')}
                    type="info"
                    showIcon
                    style={{ marginBottom: 16 }}
                  />
                  <Space wrap>
                    <Button icon={<ExportOutlined />} onClick={() => handleDsarRequest('access')}>
                      {t('privacy.request_access')}
                    </Button>
                    <Button icon={<EuroCircleOutlined />} onClick={() => handleDsarRequest('export')}>
                      {t('privacy.request_export')}
                    </Button>
                    <Button icon={<SafetyOutlined />} onClick={() => handleDsarRequest('restriction')}>
                      {t('privacy.request_restriction')}
                    </Button>
                    <Button icon={<DeleteOutlined />} danger onClick={() => handleDsarRequest('withdraw-consent')}>
                      {t('privacy.withdraw_consent')}
                    </Button>
                  </Space>

                  <Divider>{t('privacy.request_history')}</Divider>
                  <Table
                    dataSource={dsarRequests}
                    columns={dsarColumns}
                    rowKey="id"
                    pagination={{ pageSize: 5 }}
                    locale={{ emptyText: t('privacy.no_requests') }}
                  />
                </Card>
              ),
            },
            {
              key: 'privacy',
              label: t('privacy.settings'),
              children: (
                <Card title={t('privacy.privacy_settings')}>
                  <List
                    itemLayout="horizontal"
                    dataSource={[
                      {
                        title: t('privacy.do_not_sell_share'),
                        description: t('privacy.do_not_sell_share_desc'),
                      checked: privacySettings?.do_not_sell_share ?? true,
                        onChange: (v: boolean) => handlePrivacySettings({ do_not_sell_share: v }),
                      },
                      {
                        title: t('privacy.cross_border'),
                        description: t('privacy.cross_border_desc'),
                      checked: privacySettings?.cross_border_disclosure_given ?? false,
                        onChange: (v: boolean) => handlePrivacySettings({ cross_border_disclosure_given: v }),
                      },
                    ]}
                    renderItem={(item) => (
                      <List.Item
                        actions={[
                          <Switch
                            key="switch"
                            checked={item.checked}
                            onChange={item.onChange}
                          />,
                        ]}
                      >
                        <List.Item.Meta
                          title={item.title}
                          description={item.description}
                        />
                      </List.Item>
                    )}
                  />

                  <Divider />

                  <Title level={5}>{t('privacy.data_location')}</Title>
                  <Select
                    style={{ width: 200 }}
                    value={privacySettings?.data_location_preference || 'global'}
                    onChange={(v) => handlePrivacySettings({ data_location_preference: v })}
                  >
                    <Option value="global">{t('privacy.location_global')}</Option>
                    <Option value="us">{t('privacy.location_us')}</Option>
                    <Option value="eu">{t('privacy.location_eu')}</Option>
                    <Option value="tw">{t('privacy.location_tw')}</Option>
                  </Select>
                </Card>
              ),
            },
            {
              key: 'age',
              label: t('privacy.age_verification'),
              children: (
                <Card title={t('privacy.age_title')}>
                  <Alert
                    message={t('privacy.age_info')}
                    type="info"
                    showIcon
                    style={{ marginBottom: 16 }}
                  />
                  <Space direction="vertical">
                    <Checkbox
                      checked={ageVerification?.is_minor || false}
                      onChange={(e) => handleAgeVerification(e.target.checked)}
                    >
                      {t('privacy.i_am_minor')}
                    </Checkbox>
                    {ageVerification?.is_minor && (
                      <Text type="secondary">{t('privacy.guardian_consent_required')}</Text>
                    )}
                  </Space>

                  {ageVerification?.is_minor && (
                    <>
                      <Divider />
                      <Input.Search
                        placeholder={t('privacy.guardian_email')}
                        enterButton={t('privacy.provide_guardian')}
                        onSearch={(email) => handleAgeVerification(true, email)}
                      />
                    </>
                  )}

                  {ageVerification?.age_verified_at && (
                    <div style={{ marginTop: 16 }}>
                      <Text type="success">
                        {t('privacy.verified_at')}: {ageVerification.age_verified_at}
                      </Text>
                    </div>
                  )}
                </Card>
              ),
            },
            {
              key: 'retention',
              label: t('privacy.retention'),
              children: (
                <Card
                  title={t('privacy.retention_title')}
                  extra={
                    <Button onClick={handleRunPurge}>{t('privacy.run_purge')}</Button>
                  }
                >
                  <Alert
                    message={t('privacy.retention_info')}
                    type="info"
                    showIcon
                    style={{ marginBottom: 16 }}
                  />
                  <Table
                    dataSource={retentionPolicies}
                    columns={retentionColumns}
                    rowKey="id"
                    pagination={false}
                    locale={{ emptyText: t('privacy.no_policies') }}
                  />
                </Card>
              ),
            },
          ]}
        />
      </Space>
    </div>
  );
};

export default PrivacyPage;