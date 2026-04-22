import React, { useEffect, useState } from 'react';
import {
  Alert,
  Button,
  Card,
  Col,
  Form,
  Input,
  Radio,
  Row,
  Select,
  Space,
  Switch,
  Typography,
  message,
} from 'antd';
import { PlusOutlined, SettingOutlined } from '@ant-design/icons';
import axios from 'axios';
import { useTranslation } from 'react-i18next';
import { useAuth } from '../AuthContext';
import { GUEST_NAME_KEY } from '../Chat';

const { Title } = Typography;

const SettingsPage: React.FC = () => {
  const { t } = useTranslation();
  const { user, refreshUser } = useAuth();
  const [loading, setLoading] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [availableModels, setAvailableModels] = useState<string[]>(['gpt-4o-mini', 'gpt-4o', 'gpt-4-turbo']);
  const [form] = Form.useForm();

  useEffect(() => {
    axios.get('/api/ai/models').then(res => {
      if (res.data?.available_models?.length) {
        setAvailableModels(res.data.available_models);
      }
    }).catch(() => {});
  }, []);

  useEffect(() => {
    if (user) {
      let pdfPasswords: string[] = [];
      if (user.pdf_passwords) {
        try {
          const parsed = JSON.parse(user.pdf_passwords);
          if (Array.isArray(parsed)) {
            pdfPasswords = parsed;
          }
        } catch {
          pdfPasswords = [];
        }
      }

      form.setFieldsValue({
        display_name: user.display_name,
        auto_reply: user.auto_reply,
        dry_run: user.dry_run,
        email_format: user.email_format,
        mail_send_method: user.mail_send_method || 'direct_mx',
        training_data_consent: !!user.training_data_consent,
        timezone: user.timezone || 'UTC',
        preferred_language: user.preferred_language || 'en',
        assistant_name_zh: user.assistant_name_zh,
        assistant_name_en: user.assistant_name_en,
        assistant_tone_zh: user.assistant_tone_zh,
        assistant_tone_en: user.assistant_tone_en,
        pdf_passwords: pdfPasswords,
        preferred_ai_model: user.preferred_ai_model || 'gpt-4o-mini',
        data_retention_days: user.data_retention_days || 365,
        do_not_sell: !!user.do_not_sell,
      });
    } else {
      form.setFieldsValue({
        display_name: localStorage.getItem(GUEST_NAME_KEY) || '',
      });
    }
  }, [user, form]);

  const onFinish = async (values: any) => {
    if (!user) {
      localStorage.setItem(GUEST_NAME_KEY, values.display_name || '');
      message.success('Guest settings saved locally!');
      return;
    }

    setLoading(true);
    try {
      await axios.post('/api/settings', { email: user.email, ...values });
      // Handle do_not_sell separately
      await axios.post('/api/privacy/do-not-sell', { email: user.email, do_not_sell: !!values.do_not_sell });
      message.success('Settings saved successfully!');
      refreshUser();
    } catch {
      message.error('Failed to save settings.');
    } finally {
      setLoading(false);
    }
  };

  const requestDataDeletion = async () => {
    if (!user) return;
    setDeleting(true);
    try {
      const res = await axios.post('/api/data-deletion/request', { email: user.email });
      if (res.data?.status === 'success') {
        message.success('Deletion request submitted. Please check your email for the confirmation link.');
      } else {
        message.error(res.data?.message || 'Failed to request data deletion.');
      }
    } catch {
      message.error('Failed to request data deletion.');
    } finally {
      setDeleting(false);
    }
  };

  return (
    <Card title={t('system_settings')} bordered={false} style={{ maxWidth: 650, margin: '0 auto' }}>
      {!user && (
        <Alert
          message="Guest Mode"
          description="You are currently not logged in. Settings like your nickname will be stored only in this browser."
          type="info"
          showIcon
          style={{ marginBottom: 20 }}
        />
      )}
      <Form form={form} layout="vertical" onFinish={onFinish}>
        <Title level={5} style={{ marginBottom: 16 }}>Personal Info</Title>
        <Form.Item name="display_name" label="Your Display Name (顯示名稱)" tooltip="How the AI assistant and system should call you.">
          <Input placeholder="e.g. Yihua, Master, etc." />
        </Form.Item>

        {user && (
          <>
            <Title level={5} style={{ margin: '24px 0 16px' }}>{t('assistant_identity')}</Title>
            <Typography.Paragraph style={{ color: '#86868b', fontSize: '12px', marginBottom: 16 }}>{t('identity_desc')}</Typography.Paragraph>
            <Row gutter={16}>
              <Col span={12}>
                <Form.Item name="assistant_name_zh" label={t('assistant_name_zh')}>
                  <Input placeholder="e.g. 小管家" />
                </Form.Item>
              </Col>
              <Col span={12}>
                <Form.Item name="assistant_name_en" label={t('assistant_name_en')}>
                  <Input placeholder="e.g. Butler" />
                </Form.Item>
              </Col>
            </Row>
            <Row gutter={16}>
              <Col span={12}>
                <Form.Item name="assistant_tone_zh" label={t('assistant_tone_zh')} tooltip="設定中文回覆時的語氣，例如：專業、親切、簡短、幽默。">
                  <Input placeholder="e.g. 專業且快速" />
                </Form.Item>
              </Col>
              <Col span={12}>
                <Form.Item name="assistant_tone_en" label={t('assistant_tone_en')} tooltip="Set the tone for English replies, e.g. professional, friendly, concise, witty.">
                  <Input placeholder="e.g. witty and brief" />
                </Form.Item>
              </Col>
            </Row>

            <Title level={5} style={{ margin: '24px 0 16px' }}>AI Model</Title>
            <Form.Item
              name="preferred_ai_model"
              label="Preferred AI Model"
              tooltip="Select which AI model to use for chat replies."
            >
              <Select
                options={availableModels.map(m => ({ value: m, label: m }))}
              />
            </Form.Item>

            <Title level={5} style={{ margin: '24px 0 16px' }}>{t('processing_preferences')}</Title>
            <Form.Item name="dry_run" label="Dry Run Mode (試運行模式)" valuePropName="checked" tooltip="When enabled, AI replies are drafted and sent to your own email for review. When disabled, they are sent directly to the original sender.">
              <Switch />
            </Form.Item>
            <Form.Item name="auto_reply" label="Auto Reply (自動回覆)" valuePropName="checked" tooltip="Automatically send the AI-generated reply. If Dry Run is off, this will send it to the external sender.">
              <Switch />
            </Form.Item>
            <Form.Item name="email_format" label={t('email_format')} tooltip="Choose how magic links and notifications are sent to you.">
              <Radio.Group>
                <Radio value="both">{t('format_both')}</Radio>
                <Radio value="html">{t('format_html')}</Radio>
                <Radio value="plain">{t('format_plain')}</Radio>
              </Radio.Group>
            </Form.Item>

            <Form.Item
              name="mail_send_method"
              label="Mail Send Method"
              tooltip="Preferred delivery method for outgoing emails. The system will automatically fall back to the other method if delivery fails."
              rules={[{ required: true, message: 'Please select a mail send method' }]}
            >
              <Radio.Group>
                <Radio value="direct_mx">Direct MX (default)</Radio>
                <Radio value="relay">SMTP Relay</Radio>
              </Radio.Group>
            </Form.Item>

            <Form.Item
              name="training_data_consent"
              label={t('training_data_consent')}
              valuePropName="checked"
              tooltip={t('training_data_consent_desc')}
            >
              <Switch />
            </Form.Item>
            <Typography.Paragraph style={{ color: '#86868b', fontSize: '12px', marginTop: -8 }}>
              {t('training_data_consent_note')}
            </Typography.Paragraph>

            <Form.Item
              name="preferred_language"
              label={t('preferred_language')}
              tooltip="Language used for system-generated emails sent to you."
              rules={[{ required: true, message: 'Please select a language' }]}
            >
              <Select
                options={[
                  { value: 'en', label: 'English' },
                  { value: 'zh-TW', label: '繁體中文' },
                ]}
              />
            </Form.Item>

            <Form.Item
              name="timezone"
              label="Timezone"
              tooltip="Used to display local times in Dashboard."
              rules={[{ required: true, message: 'Please select a timezone' }]}
            >
              <Select
                showSearch
                options={[
                  { value: 'UTC', label: 'UTC' },
                  { value: 'Asia/Taipei', label: 'Asia/Taipei (UTC+8)' },
                  { value: 'Asia/Tokyo', label: 'Asia/Tokyo (UTC+9)' },
                  { value: 'Asia/Shanghai', label: 'Asia/Shanghai (UTC+8)' },
                  { value: 'Asia/Singapore', label: 'Asia/Singapore (UTC+8)' },
                  { value: 'America/Los_Angeles', label: 'America/Los_Angeles' },
                  { value: 'America/New_York', label: 'America/New_York' },
                  { value: 'Europe/London', label: 'Europe/London' },
                  { value: 'Europe/Berlin', label: 'Europe/Berlin' },
                ]}
              />
            </Form.Item>

            <Title level={5} style={{ margin: '24px 0 16px' }}>Privacy & Compliance</Title>
            <Form.Item
              name="do_not_sell"
              label="Do Not Sell/Share My Data"
              valuePropName="checked"
              tooltip="Opt out of the sale or sharing of your personal information (US state privacy laws)."
            >
              <Switch />
            </Form.Item>
            <Form.Item
              name="data_retention_days"
              label="Data Retention Period (days)"
              tooltip="How many days to keep your chat transcripts, feedback, and logs. Min 30, max 3650."
            >
              <Select
                options={[
                  { value: 30, label: '30 days' },
                  { value: 90, label: '90 days' },
                  { value: 180, label: '180 days' },
                  { value: 365, label: '1 year (default)' },
                  { value: 730, label: '2 years' },
                  { value: 1825, label: '5 years' },
                  { value: 3650, label: '10 years' },
                ]}
              />
            </Form.Item>
            <Typography.Paragraph style={{ color: '#86868b', fontSize: '12px', marginBottom: 8 }}>
              Note: Users under 13 (COPPA) require verifiable guardian consent before using this service.
            </Typography.Paragraph>

            <Title level={5} style={{ margin: '24px 0 16px' }}>PDF Passwords</Title>
            <Typography.Paragraph style={{ color: '#86868b', fontSize: '12px', marginBottom: 16 }}>
              If some PDF attachments are encrypted, add the passwords here so the system can decode and convert them into Markdown.
            </Typography.Paragraph>
            <Form.List name="pdf_passwords">
              {(fields, { add, remove }) => (
                <>
                  {fields.map((field) => (
                    <Space key={field.key} style={{ display: 'flex', marginBottom: 8 }} align="baseline">
                      <Form.Item
                        {...field}
                        style={{ marginBottom: 0, minWidth: 340 }}
                        rules={[{ required: true, message: 'Password cannot be empty' }]}
                      >
                        <Input.Password placeholder="PDF password" />
                      </Form.Item>
                      <Button onClick={() => remove(field.name)} danger>
                        Remove
                      </Button>
                    </Space>
                  ))}
                  <Form.Item>
                    <Button type="dashed" onClick={() => add()} icon={<PlusOutlined />}>
                      Add PDF Password
                    </Button>
                  </Form.Item>
                </>
              )}
            </Form.List>
          </>
        )}

        <Form.Item style={{ marginTop: 32 }}>
          <Button type="primary" htmlType="submit" loading={loading} icon={<SettingOutlined />} block>
            Save Settings
          </Button>
        </Form.Item>

        {user && (
          <Card size="small" title="GDPR / Data Deletion" style={{ marginTop: 12, borderColor: '#ffd8bf' }}>
            <Typography.Paragraph style={{ marginBottom: 12, color: '#8c8c8c' }}>
              You can request deletion of all your data. The system will email you a confirmation link with a data summary report. After opening the link, you must confirm again before final deletion.
            </Typography.Paragraph>
            <Button danger loading={deleting} onClick={requestDataDeletion}>
              Request Deletion of All My Data
            </Button>
          </Card>
        )}
      </Form>
    </Card>
  );
};

export default SettingsPage;
