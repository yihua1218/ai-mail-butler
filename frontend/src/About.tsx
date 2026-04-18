import React, { useEffect, useState } from 'react';
import { Card, Descriptions, Typography, Spin, Alert, Tag } from 'antd';
import axios from 'axios';

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
}

export const About: React.FC = () => {
  const [info, setInfo] = useState<BuildInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

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

  return (
    <div>
      <div style={{ marginBottom: 32 }}>
        <Title level={2}>About System</Title>
        <Typography.Paragraph style={{ color: '#86868b', fontSize: '16px' }}>
          Detailed system and compilation environment information.
        </Typography.Paragraph>
      </div>

      <Card bordered={false} hoverable style={{ borderRadius: 12, marginBottom: 24 }}>
        <Descriptions
          title="Build Environment Fingerprint"
          bordered
          column={{ xxl: 2, xl: 2, lg: 2, md: 1, sm: 1, xs: 1 }}
        >
          <Descriptions.Item label="App Version">
            <Tag color="blue">v{info?.version}</Tag>
          </Descriptions.Item>
          <Descriptions.Item label="Git Commit">
            <code style={{ fontFamily: 'monospace', background: '#f5f5f7', padding: '2px 8px', borderRadius: 4 }}>
              {info?.git_commit}
            </code>
          </Descriptions.Item>
          <Descriptions.Item label="Build Profile">
            <Tag color={profileColor}>{info?.profile}</Tag>
          </Descriptions.Item>
          <Descriptions.Item label="Build Date (UTC)">{info?.build_date}</Descriptions.Item>
          <Descriptions.Item label="Target Architecture">{info?.target}</Descriptions.Item>
          <Descriptions.Item label="Host Compiler">{info?.host}</Descriptions.Item>
        </Descriptions>
      </Card>

      <Card bordered={false} hoverable style={{ borderRadius: 12 }}>
        <Descriptions
          title="Build Machine Hardware"
          bordered
          column={{ xxl: 2, xl: 2, lg: 2, md: 1, sm: 1, xs: 1 }}
        >
          <Descriptions.Item label="CPU Model" span={2}>
            {info?.build_cpu_model}
          </Descriptions.Item>
          <Descriptions.Item label="CPU Logical Cores">
            {info?.build_cpu_cores} cores
          </Descriptions.Item>
          <Descriptions.Item label="Total RAM">
            {info?.build_ram}
          </Descriptions.Item>
          <Descriptions.Item label="Root Disk Size">
            {info?.build_disk}
          </Descriptions.Item>
        </Descriptions>
      </Card>
    </div>
  );
};
