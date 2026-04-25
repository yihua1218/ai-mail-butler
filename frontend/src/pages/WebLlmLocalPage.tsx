import React from 'react';
import { Alert, Card, Col, Divider, List, Row, Tag, Typography } from 'antd';
import {
  CloudDownloadOutlined,
  SafetyOutlined,
  ThunderboltOutlined,
  WarningOutlined,
} from '@ant-design/icons';
import { useTranslation } from 'react-i18next';

const { Title, Paragraph, Text } = Typography;

const WebLlmLocalPage: React.FC = () => {
  const { i18n } = useTranslation();
  const isZh = i18n.language === 'zh-TW';

  const requirements = isZh
    ? [
        '電腦必須保持開機，且瀏覽器（Chrome / Edge）需持續開啟。',
        '瀏覽器與電腦不可進入睡眠、休眠、強制省電或被系統回收背景程序。',
        '使用者必須持續登入 Gmail 網頁版，並授予 Extension 必要權限。',
        '首次模型下載與預熱需要時間，WebGPU 不可用時僅能降級為人工審閱模式。',
      ]
    : [
        'The computer must stay powered on and Chrome/Edge must remain open.',
        'The browser and device cannot enter sleep/hibernate or aggressively suspend background tasks.',
        'The user must remain signed in to Gmail Web and grant required extension permissions.',
        'Model download and warmup take time; if WebGPU is unavailable, fallback is review-only mode.',
      ];

  const localOnlyScopes = isZh
    ? [
        '規則、語氣模板、允許清單、審核門檻與審計摘要都儲存在瀏覽器本機（chrome.storage.local + IndexedDB）。',
        '郵件內容不傳送到第三方 AI 推論服務。',
        'Extension 不依賴本專案伺服器來儲存使用者郵件、自動化規則或推論結果。',
        '使用者可在 Side Panel 進行本機資料清除與停用自動化。',
      ]
    : [
        'Rules, tone profiles, allowlists, thresholds, and audit metadata stay in browser-local storage.',
        'Email content is not forwarded to third-party AI inference services.',
        'The extension does not use this project server to store user mail, rules, or inference outputs.',
        'Users can wipe local data and disable automation from the side panel.',
      ];

  const flow = isZh
    ? [
        'Content Script 讀取可見 Gmail thread。',
        '背景 Service Worker 在本機比對使用者規則。',
        'WebLLM 在本地端產生回覆與信心分數。',
        '策略層判斷：忽略 / 建立草稿 / 條件式自動寄送。',
        '所有決策摘要寫入本機審計記錄（預設不保存完整郵件明文）。',
      ]
    : [
        'Content script reads visible Gmail threads.',
        'Background service worker matches user-defined local rules.',
        'WebLLM generates local reply text and confidence score.',
        'Policy layer decides: ignore / create draft / guarded auto-send.',
        'Decision metadata is logged locally (without full plaintext by default).',
      ];

  return (
    <div style={{ maxWidth: 1160, margin: '0 auto' }}>
      <Title level={2} style={{ marginBottom: 8 }}>
        {isZh ? 'WebLLM 本地端 Gmail 自動回覆' : 'WebLLM Local Gmail Auto Reply'}
      </Title>
      <Paragraph type="secondary" style={{ marginBottom: 20 }}>
        {isZh
          ? '此模式透過 Browser Extension 在你的瀏覽器本地端執行 WebLLM、規則引擎與回覆策略。'
          : 'This mode runs WebLLM, rule matching, and reply policies locally in your browser via extension.'}
      </Paragraph>

      <Alert
        type="warning"
        showIcon
        icon={<WarningOutlined />}
        message={isZh ? '運作限制（請先完整閱讀）' : 'Operational Limits (Read Before Use)'}
        description={
          <List
            size="small"
            dataSource={requirements}
            renderItem={(item) => <List.Item>{item}</List.Item>}
            style={{ marginTop: 8 }}
          />
        }
        style={{ marginBottom: 20, borderRadius: 12 }}
      />

      <Alert
        type="info"
        showIcon
        message={isZh ? '權限檢查與限制提示' : 'Permission Check & Limitation Notice'}
        description={
          isZh
            ? '若 Extension 權限或瀏覽器 API 不可用，請開啟 Extension side panel 的「Permission & Capability Check」。系統會列出缺少項目與可用功能限制。'
            : 'If extension permissions or browser APIs are unavailable, open the extension side panel and check "Permission & Capability Check" for missing items and feature limitations.'
        }
        style={{ marginBottom: 20, borderRadius: 12 }}
      />

      <Row gutter={[16, 16]}>
        <Col xs={24} lg={8}>
          <Card
            title={isZh ? '本地端資料邊界' : 'Local Data Boundary'}
            extra={<SafetyOutlined />}
            style={{ height: '100%', borderRadius: 14 }}
          >
            <List
              size="small"
              dataSource={localOnlyScopes}
              renderItem={(item) => <List.Item>{item}</List.Item>}
            />
          </Card>
        </Col>

        <Col xs={24} lg={8}>
          <Card
            title={isZh ? '執行流程' : 'Execution Pipeline'}
            extra={<ThunderboltOutlined />}
            style={{ height: '100%', borderRadius: 14 }}
          >
            <List
              size="small"
              dataSource={flow}
              renderItem={(item, idx) => (
                <List.Item>
                  <Text strong style={{ marginRight: 8 }}>{idx + 1}.</Text>
                  {item}
                </List.Item>
              )}
            />
          </Card>
        </Col>

        <Col xs={24} lg={8}>
          <Card
            title={isZh ? '部署與安裝' : 'Install & Deploy'}
            extra={<CloudDownloadOutlined />}
            style={{ height: '100%', borderRadius: 14 }}
          >
            <Paragraph>
              {isZh
                ? '本專案已建立 Chrome Extension 骨架於 browser-extension/ 目錄，可直接以「開發人員模式」載入。'
                : 'A Chrome extension scaffold is included in browser-extension/ and can be loaded in Developer Mode.'}
            </Paragraph>
            <Divider style={{ margin: '12px 0' }} />
            <Tag color="blue">chrome.storage.local</Tag>
            <Tag color="blue">IndexedDB</Tag>
            <Tag color="green">Local-only Processing</Tag>
            <Tag color="orange">Guarded Auto-send</Tag>
          </Card>
        </Col>
      </Row>

      <Card style={{ marginTop: 20, borderRadius: 14 }}>
        <Title level={4} style={{ marginBottom: 8 }}>
          {isZh ? '建議啟用順序' : 'Recommended Rollout'}
        </Title>
        <List
          size="small"
          dataSource={
            isZh
              ? [
                  '先啟用「草稿輔助」並觀察規則命中率與模型回覆品質。',
                  '確認敏感詞與風險攔截設定後，再開啟有限條件的自動寄送。',
                  '僅針對低風險、高重複性的寄件者或主題開啟 guarded auto-send。',
                ]
              : [
                  'Start with draft-assist mode and validate match quality.',
                  'Enable auto-send only after sensitive-topic and veto filters are stable.',
                  'Allow guarded auto-send only for low-risk repetitive email classes.',
                ]
          }
          renderItem={(item) => <List.Item>{item}</List.Item>}
        />
      </Card>
    </div>
  );
};

export default WebLlmLocalPage;
