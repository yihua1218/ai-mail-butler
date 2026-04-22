import React, { useEffect, useState } from 'react';
import { Card, Descriptions, Typography, Spin, Alert, Tag, Button, Space, Tooltip } from 'antd';
import { CheckOutlined, CopyOutlined } from '@ant-design/icons';
import axios from 'axios';
import { useTranslation } from 'react-i18next';

const { Title } = Typography;

interface BuildInfo {
  version: string;
  target: string;
  host: string;
  profile: string;
  git_commit: string;
  build_date: string;
  build_cpu_cores: string;
  build_cpu_model: string;
  build_ram: string;
  build_disk: string;
  assistant_email: string;
}

export const About: React.FC = () => {
  const { i18n } = useTranslation();
  const [info, setInfo] = useState<BuildInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    axios.get('/api/about')
      .then(res => {
        setInfo(res.data);
        setLoading(false);
      })
      .catch(err => {
        setError(err.message);
        setLoading(false);
      });
  }, []);

  if (loading) return <Spin style={{ display: 'block', margin: '100px auto' }} size="large" />;
  if (error) return <Alert message="Error" description={error} type="error" showIcon />;

  const profileColor = info?.profile === 'release' ? 'green' : 'orange';
  const isZh = i18n.language === 'zh-TW';

  const handleCopy = async () => {
    const email = info?.assistant_email;
    if (!email) return;
    try {
      await navigator.clipboard.writeText(email);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1500);
    } catch {
      setCopied(false);
    }
  };

  const text = isZh
    ? {
        pageTitle: '系統資訊',
        pageDesc: '系統建置資訊，以及 AI 郵件助理的功能範圍與使用限制。',
        scopeTitle: 'AI 郵件助理功能範圍與限制',
        mailbox: '助理收信信箱',
        scope: '可協助：轉寄郵件處理、規則設定、回覆草稿/寄送偏好、Dashboard 與 Log 判讀。',
        limits: '限制：不提供寫程式、非郵件助理主題的問題解答；可短暫閒聊，但會引導回郵件處理工作。',
        gdpr: 'GDPR 宣告：使用者可要求刪除個人資料，系統會提供刪除前統計報表與二次確認流程。',
        buildEnvTitle: '建置環境指紋',
        machineTitle: '建置機器硬體資訊',
        appVersion: '應用程式版本',
        gitCommit: 'Git Commit',
        buildProfile: '建置模式',
        buildDate: '建置時間 (UTC)',
        targetArch: '目標架構',
        hostCompiler: '編譯主機',
        cpuModel: 'CPU 型號',
        cpuCores: 'CPU 邏輯核心數',
        totalRam: '總記憶體',
        disk: '系統磁碟容量',
      }
    : {
        pageTitle: 'About System',
        pageDesc: 'System build information, plus AI email assistant scope and limitations.',
        scopeTitle: 'AI Email Assistant Scope & Limits',
        mailbox: 'Assistant Mailbox Address',
        scope: 'Supported: forwarded-email processing, rule setup, draft/sending preferences, dashboard and log interpretation.',
        limits: 'Limits: no coding help and no unrelated Q&A; brief casual chat is allowed but the assistant will guide back to email workflow tasks.',
        gdpr: 'GDPR declaration: users can request personal data deletion with a pre-deletion summary report and a required second confirmation step.',
        buildEnvTitle: 'Build Environment Fingerprint',
        machineTitle: 'Build Machine Hardware',
        appVersion: 'App Version',
        gitCommit: 'Git Commit',
        buildProfile: 'Build Profile',
        buildDate: 'Build Date (UTC)',
        targetArch: 'Target Architecture',
        hostCompiler: 'Host Compiler',
        cpuModel: 'CPU Model',
        cpuCores: 'CPU Logical Cores',
        totalRam: 'Total RAM',
        disk: 'Root Disk Size',
      };

  return (
    <div>
      <div style={{ marginBottom: 32 }}>
        <Title level={2}>{text.pageTitle}</Title>
        <Typography.Paragraph style={{ color: '#86868b', fontSize: '16px' }}>
          {text.pageDesc}
        </Typography.Paragraph>
      </div>

      <Card bordered={false} hoverable style={{ borderRadius: 12, marginBottom: 24 }}>
        <Descriptions
          title={text.scopeTitle}
          bordered
          column={{ xxl: 1, xl: 1, lg: 1, md: 1, sm: 1, xs: 1 }}
        >
          <Descriptions.Item label={text.mailbox}>
            <Space size={4}>
              <code style={{ fontFamily: 'monospace', background: '#f5f5f7', padding: '2px 8px', borderRadius: 4 }}>
                {info?.assistant_email}
              </code>
              <Tooltip title={copied ? (isZh ? '已複製' : 'Copied') : (isZh ? '複製信箱' : 'Copy mailbox')}>
                <Button
                  type="text"
                  aria-label={isZh ? '複製助理信箱' : 'Copy assistant mailbox'}
                  icon={copied ? <CheckOutlined /> : <CopyOutlined />}
                  onClick={handleCopy}
                />
              </Tooltip>
            </Space>
          </Descriptions.Item>
          <Descriptions.Item label={isZh ? '功能範圍' : 'Scope'}>{text.scope}</Descriptions.Item>
          <Descriptions.Item label={isZh ? '使用限制' : 'Limitations'}>{text.limits}</Descriptions.Item>
          <Descriptions.Item label="GDPR">{text.gdpr}</Descriptions.Item>
        </Descriptions>
      </Card>

      <Card bordered={false} hoverable style={{ borderRadius: 12, marginBottom: 24 }}>
        <Descriptions
          title={text.buildEnvTitle}
          bordered
          column={{ xxl: 2, xl: 2, lg: 2, md: 1, sm: 1, xs: 1 }}
        >
          <Descriptions.Item label={text.appVersion}>
            <Tag color="blue">v{info?.version}</Tag>
          </Descriptions.Item>
          <Descriptions.Item label={text.gitCommit}>
            <code style={{ fontFamily: 'monospace', background: '#f5f5f7', padding: '2px 8px', borderRadius: 4 }}>
              {info?.git_commit}
            </code>
          </Descriptions.Item>
          <Descriptions.Item label={text.buildProfile}>
            <Tag color={profileColor}>{info?.profile}</Tag>
          </Descriptions.Item>
          <Descriptions.Item label={text.buildDate}>{info?.build_date}</Descriptions.Item>
          <Descriptions.Item label={text.targetArch}>{info?.target}</Descriptions.Item>
          <Descriptions.Item label={text.hostCompiler}>{info?.host}</Descriptions.Item>
        </Descriptions>
      </Card>

      <Card bordered={false} hoverable style={{ borderRadius: 12 }}>
        <Descriptions
          title={text.machineTitle}
          bordered
          column={{ xxl: 2, xl: 2, lg: 2, md: 1, sm: 1, xs: 1 }}
        >
          <Descriptions.Item label={text.cpuModel} span={2}>
            {info?.build_cpu_model}
          </Descriptions.Item>
          <Descriptions.Item label={text.cpuCores}>
            {info?.build_cpu_cores} cores
          </Descriptions.Item>
          <Descriptions.Item label={text.totalRam}>
            {info?.build_ram}
          </Descriptions.Item>
          <Descriptions.Item label={text.disk}>
            {info?.build_disk}
          </Descriptions.Item>
        </Descriptions>
      </Card>
    </div>
  );
};
