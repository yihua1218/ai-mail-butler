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
  const [form] = Form.useForm();

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
        rule_label_mode: user.rule_label_mode || 'ai_first',
        training_data_consent: !!user.training_data_consent,
        timezone: user.timezone || 'UTC',
        preferred_language: user.preferred_language || 'en',
        time_format: user.time_format || '24h',
        date_format: user.date_format || 'auto',
        assistant_name_zh: user.assistant_name_zh,
        assistant_name_en: user.assistant_name_en,
        assistant_tone_zh: user.assistant_tone_zh,
        assistant_tone_en: user.assistant_tone_en,
        pdf_passwords: pdfPasswords,
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
      message.success(t('guest_settings_saved'));
      return;
    }

    setLoading(true);
    try {
      await axios.post('/api/settings', { email: user.email, ...values });
      message.success(t('settings_saved'));
      refreshUser();
    } catch {
      message.error(t('settings_save_failed'));
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
        message.success(t('deletion_requested'));
      } else {
        message.error(res.data?.message || t('deletion_request_failed'));
      }
    } catch {
      message.error(t('deletion_request_failed'));
    } finally {
      setDeleting(false);
    }
  };

  return (
    <Card title={t('system_settings')} bordered={false} style={{ margin: '0 auto' }}>
      {!user && (
        <Alert
          message={t('guest_mode_title')}
          description={t('guest_mode_desc')}
          type="info"
          showIcon
          style={{ marginBottom: 20 }}
        />
      )}
      <Form form={form} layout="vertical" onFinish={onFinish}>
        <Title level={5} style={{ marginBottom: 16 }}>{t('personal_info')}</Title>
        <Form.Item name="display_name" label={t('display_name_label')} tooltip={t('display_name_tooltip')}>
          <Input placeholder={t('display_name_placeholder')} />
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
                <Form.Item name="assistant_tone_zh" label={t('assistant_tone_zh')} tooltip={t('assistant_tone_zh_tooltip')}>
                  <Input placeholder="e.g. 專業且快速" />
                </Form.Item>
              </Col>
              <Col span={12}>
                <Form.Item name="assistant_tone_en" label={t('assistant_tone_en')} tooltip={t('assistant_tone_en_tooltip')}>
                  <Input placeholder="e.g. witty and brief" />
                </Form.Item>
              </Col>
            </Row>

            <Title level={5} style={{ margin: '24px 0 16px' }}>{t('processing_preferences')}</Title>
            <Form.Item name="dry_run" label={t('dry_run_label')} valuePropName="checked" tooltip={t('dry_run_tooltip')}>
              <Switch />
            </Form.Item>
            <Form.Item name="auto_reply" label={t('auto_reply_label')} valuePropName="checked" tooltip={t('auto_reply_tooltip')}>
              <Switch />
            </Form.Item>
            <Form.Item name="email_format" label={t('email_format')} tooltip={t('email_format_tooltip')}>
              <Radio.Group>
                <Radio value="both">{t('format_both')}</Radio>
                <Radio value="html">{t('format_html')}</Radio>
                <Radio value="plain">{t('format_plain')}</Radio>
              </Radio.Group>
            </Form.Item>

            <Form.Item
              name="mail_send_method"
              label={t('mail_send_method_label')}
              tooltip={t('mail_send_method_tooltip')}
              rules={[{ required: true, message: t('mail_send_method_required') }]}
            >
              <Radio.Group>
                <Radio value="direct_mx">Direct MX (default)</Radio>
                <Radio value="relay">SMTP Relay</Radio>
              </Radio.Group>
            </Form.Item>

            <Form.Item
              name="rule_label_mode"
              label={t('rule_label_mode_label')}
              tooltip={t('rule_label_mode_tooltip')}
              rules={[{ required: true, message: t('rule_label_mode_required') }]}
            >
              <Radio.Group>
                <Radio value="ai_first">{t('rule_label_mode_ai_first')}</Radio>
                <Radio value="deterministic_only">{t('rule_label_mode_deterministic')}</Radio>
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
              tooltip={t('preferred_language_tooltip')}
              rules={[{ required: true, message: t('preferred_language_required') }]}
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
              label={t('timezone_label')}
              tooltip={t('timezone_tooltip')}
              rules={[{ required: true, message: t('timezone_required') }]}
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

            <Form.Item
              name="time_format"
              label={t('time_format_label')}
              tooltip={t('time_format_tooltip')}
            >
              <Radio.Group>
                <Radio value="24h">{t('time_format_24h')}</Radio>
                <Radio value="12h">{t('time_format_12h')}</Radio>
              </Radio.Group>
            </Form.Item>

            <Form.Item
              name="date_format"
              label={t('date_format_label')}
              tooltip={t('date_format_tooltip')}
            >
              <Select
                options={[
                  { value: 'auto', label: t('date_format_auto') },
                  { value: 'tw', label: t('date_format_tw') },
                  { value: 'iso', label: t('date_format_iso') },
                  { value: 'us', label: t('date_format_us') },
                  { value: 'eu', label: t('date_format_eu') },
                ]}
              />
            </Form.Item>

            <Title level={5} style={{ margin: '24px 0 16px' }}>{t('pdf_passwords_title')}</Title>
            <Typography.Paragraph style={{ color: '#86868b', fontSize: '12px', marginBottom: 16 }}>
              {t('pdf_passwords_desc')}
            </Typography.Paragraph>
            <Form.List name="pdf_passwords">
              {(fields, { add, remove }) => (
                <>
                  {fields.map((field) => (
                    <Space key={field.key} style={{ display: 'flex', marginBottom: 8 }} align="baseline">
                      <Form.Item
                        {...field}
                        style={{ marginBottom: 0, minWidth: 340 }}
                        rules={[{ required: true, message: t('pdf_password_required') }]}
                      >
                        <Input.Password placeholder={t('pdf_password_placeholder')} />
                      </Form.Item>
                      <Button onClick={() => remove(field.name)} danger>
                        {t('pdf_remove_password')}
                      </Button>
                    </Space>
                  ))}
                  <Form.Item>
                    <Button type="dashed" onClick={() => add()} icon={<PlusOutlined />}>
                      {t('pdf_add_password')}
                    </Button>
                  </Form.Item>
                </>
              )}
            </Form.List>
          </>
        )}

        <Form.Item style={{ marginTop: 32 }}>
          <Button type="primary" htmlType="submit" loading={loading} icon={<SettingOutlined />} block>
            {t('save_settings')}
          </Button>
        </Form.Item>

        {user && (
          <Card size="small" title={t('gdpr_title')} style={{ marginTop: 12, borderColor: '#ffd8bf' }}>
            <Typography.Paragraph style={{ marginBottom: 12, color: '#8c8c8c' }}>
              {t('gdpr_desc')}
            </Typography.Paragraph>
            <Button danger loading={deleting} onClick={requestDataDeletion}>
              {t('gdpr_button')}
            </Button>
          </Card>
        )}
      </Form>
    </Card>
  );
};

export default SettingsPage;
