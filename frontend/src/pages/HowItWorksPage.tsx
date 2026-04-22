import React, { useEffect, useRef, useState } from 'react';
import {
  Alert,
  Badge,
  Button,
  Card,
  Col,
  Form,
  Input,
  List,
  message,
  Row,
  Spin,
  Tag,
  Tooltip,
  Typography,
} from 'antd';
import {
  BulbOutlined,
  CheckCircleFilled,
  CrownOutlined,
  LikeOutlined,
  LikeFilled,
  LockOutlined,
  MailOutlined,
  PlusOutlined,
  RobotOutlined,
  SettingOutlined,
  StarOutlined,
  ThunderboltOutlined,
} from '@ant-design/icons';
import axios from 'axios';
import { useTranslation } from 'react-i18next';
import { useAuth } from '../AuthContext';

const { Title, Paragraph, Text } = Typography;
const { TextArea } = Input;

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

interface FeatureWish {
  id: string;
  title: string;
  description?: string;
  created_by?: string;
  is_official: boolean;
  created_at: string;
  vote_count: number;
  user_has_voted: boolean;
}

// ─────────────────────────────────────────────────────────────────────────────
// Animated Flow Diagram
// ─────────────────────────────────────────────────────────────────────────────

const flowStyles = `
@keyframes fadeSlideIn {
  from { opacity: 0; transform: translateY(20px); }
  to   { opacity: 1; transform: translateY(0); }
}
@keyframes pulseBorder {
  0%, 100% { box-shadow: 0 0 0 0 rgba(0,113,227,0.35); }
  50%       { box-shadow: 0 0 0 10px rgba(0,113,227,0); }
}
@keyframes arrowFlow {
  0%   { stroke-dashoffset: 40; opacity: 0.4; }
  50%  { stroke-dashoffset: 0;  opacity: 1; }
  100% { stroke-dashoffset: 40; opacity: 0.4; }
}
@keyframes branchFadeIn {
  from { opacity: 0; transform: scale(0.92); }
  to   { opacity: 1; transform: scale(1); }
}
@keyframes shimmer {
  0%   { background-position: -200% center; }
  100% { background-position: 200% center; }
}
.flow-node {
  animation: fadeSlideIn 0.6s ease both, pulseBorder 2.4s ease-in-out 0.8s infinite;
}
.flow-arrow-path {
  stroke-dasharray: 8 4;
  animation: arrowFlow 2s linear infinite;
}
.branch-node {
  animation: branchFadeIn 0.5s ease both;
}
.branch-node.coming-soon {
  background: linear-gradient(90deg,
    rgba(0,113,227,0.08) 0%,
    rgba(88,86,214,0.16) 50%,
    rgba(0,113,227,0.08) 100%);
  background-size: 200% auto;
  animation: branchFadeIn 0.5s ease both, shimmer 3s linear infinite;
}
`;

interface FlowNodeProps {
  icon: React.ReactNode;
  label: string;
  sublabel?: string;
  delay?: number;
  color?: string;
}

const FlowNode: React.FC<FlowNodeProps> = ({ icon, label, sublabel, delay = 0, color = '#0071e3' }) => (
  <div
    className="flow-node"
    style={{
      animationDelay: `${delay}s`,
      display: 'flex',
      flexDirection: 'column',
      alignItems: 'center',
      gap: 8,
      minWidth: 90,
    }}
  >
    <div
      style={{
        width: 64,
        height: 64,
        borderRadius: 20,
        background: `linear-gradient(135deg, ${color}22, ${color}44)`,
        border: `2px solid ${color}`,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        fontSize: 28,
        color,
      }}
    >
      {icon}
    </div>
    <Text strong style={{ fontSize: 13, textAlign: 'center', lineHeight: 1.3 }}>
      {label}
    </Text>
    {sublabel && (
      <Text type="secondary" style={{ fontSize: 11, textAlign: 'center' }}>
        {sublabel}
      </Text>
    )}
  </div>
);

const AnimatedArrow: React.FC<{ delay?: number; vertical?: boolean }> = ({ delay = 0, vertical = false }) => (
  <div
    style={{
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      padding: vertical ? '4px 0' : '0 4px',
    }}
  >
    <svg
      width={vertical ? 24 : 48}
      height={vertical ? 48 : 24}
      style={{ animationDelay: `${delay}s` }}
    >
      {vertical ? (
        <>
          <path
            className="flow-arrow-path"
            d="M12 4 L12 36"
            stroke="#0071e3"
            strokeWidth="2.5"
            fill="none"
            strokeLinecap="round"
          />
          <polygon points="6,33 12,44 18,33" fill="#0071e3" opacity="0.8" />
        </>
      ) : (
        <>
          <path
            className="flow-arrow-path"
            d="M4 12 L36 12"
            stroke="#0071e3"
            strokeWidth="2.5"
            fill="none"
            strokeLinecap="round"
          />
          <polygon points="33,6 44,12 33,18" fill="#0071e3" opacity="0.8" />
        </>
      )}
    </svg>
  </div>
);

interface BranchNodeProps {
  icon: React.ReactNode;
  label: string;
  desc: string;
  delay?: number;
  isComingSoon?: boolean;
  color?: string;
}

const BranchNode: React.FC<BranchNodeProps> = ({
  icon,
  label,
  desc,
  delay = 0,
  isComingSoon = false,
  color = '#0071e3',
}) => (
  <div
    className={`branch-node${isComingSoon ? ' coming-soon' : ''}`}
    style={{
      animationDelay: `${delay}s`,
      border: `1.5px solid ${isComingSoon ? '#8e8e93' : color}`,
      borderRadius: 14,
      padding: '14px 18px',
      minWidth: 160,
      maxWidth: 200,
      textAlign: 'center',
      background: isComingSoon ? undefined : `${color}0d`,
    }}
  >
    <div style={{ fontSize: 22, marginBottom: 6, color: isComingSoon ? '#8e8e93' : color }}>{icon}</div>
    <Text strong style={{ display: 'block', fontSize: 13, color: isComingSoon ? '#8e8e93' : undefined }}>
      {label}
    </Text>
    <Text type="secondary" style={{ fontSize: 11 }}>
      {desc}
    </Text>
    {isComingSoon && (
      <Tag color="default" style={{ marginTop: 6, fontSize: 10 }}>
        Coming Soon
      </Tag>
    )}
  </div>
);

// ─────────────────────────────────────────────────────────────────────────────
// Main component
// ─────────────────────────────────────────────────────────────────────────────

const HowItWorksPage: React.FC = () => {
  const { i18n } = useTranslation();
  const { user } = useAuth();
  const isZh = i18n.language === 'zh-TW';

  const [wishes, setWishes] = useState<FeatureWish[]>([]);
  const [wishesLoading, setWishesLoading] = useState(true);
  const [submitting, setSubmitting] = useState(false);
  const [votingId, setVotingId] = useState<string | null>(null);
  const [form] = Form.useForm();
  const styleInjected = useRef(false);

  // Inject CSS animations once.
  useEffect(() => {
    if (styleInjected.current) return;
    styleInjected.current = true;
    const el = document.createElement('style');
    el.textContent = flowStyles;
    document.head.appendChild(el);
    return () => {
      document.head.removeChild(el);
    };
  }, []);

  const fetchWishes = () => {
    const email = user?.email ?? '';
    const url = email ? `/api/wishes?email=${encodeURIComponent(email)}` : '/api/wishes';
    axios
      .get<FeatureWish[]>(url)
      .then((r) => setWishes(r.data))
      .catch(() => message.error(isZh ? '載入功能許願清單失敗' : 'Failed to load wishes'))
      .finally(() => setWishesLoading(false));
  };

  useEffect(() => {
    fetchWishes();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [user?.email]);

  const handleVote = async (wish: FeatureWish) => {
    if (!user) {
      message.info(isZh ? '請先登入才能投票' : 'Please log in to vote');
      return;
    }
    setVotingId(wish.id);
    try {
      const res = await axios.post<{ voted: boolean }>(`/api/wishes/${wish.id}/vote`, {
        email: user.email,
      });
      const nowVoted = res.data.voted;
      setWishes((prev) =>
        prev.map((w) =>
          w.id === wish.id
            ? {
                ...w,
                user_has_voted: nowVoted,
                vote_count: nowVoted ? w.vote_count + 1 : w.vote_count - 1,
              }
            : w,
        ),
      );
    } catch {
      message.error(isZh ? '投票失敗，請稍後再試' : 'Vote failed, please try again');
    } finally {
      setVotingId(null);
    }
  };

  const handleSubmitWish = async (values: { title: string; description?: string }) => {
    if (!user) return;
    setSubmitting(true);
    try {
      await axios.post('/api/wishes', {
        email: user.email,
        title: values.title.trim(),
        description: values.description?.trim() || undefined,
      });
      message.success(isZh ? '已送出功能建議！' : 'Feature suggestion submitted!');
      form.resetFields();
      fetchWishes();
    } catch {
      message.error(isZh ? '送出失敗，請稍後再試' : 'Submission failed, please try again');
    } finally {
      setSubmitting(false);
    }
  };

  // ── Text content (bilingual) ──────────────────────────────────────────────

  const tx = {
    pageTitle: isZh ? 'AI 電子信箱助理 — 功能介紹' : 'AI Mail Butler — How It Works',
    pageSubtitle: isZh
      ? '轉寄規則設定完成後，AI 助理就會自動接收並處理您的信件'
      : 'Once forwarding rules are configured, your AI assistant automatically receives and processes your emails',

    // Flow nodes
    step1Label: isZh ? '您的收件匣' : 'Your Inbox',
    step1Sub: isZh ? '收到新信件' : 'New email arrives',
    step2Label: isZh ? '設定轉寄規則' : 'Configure Forwarding',
    step2Sub: isZh ? '一次設定，長期生效' : 'Set once, works forever',
    step3Label: isZh ? 'AI 助理信箱' : 'AI Mailbox',
    step3Sub: isZh ? '接收轉寄信件' : 'Receives forwarded mail',
    step4Label: isZh ? 'AI 處理引擎' : 'AI Processing Engine',
    step4Sub: isZh ? '依規則智慧處理' : 'Smart rule-based processing',

    // Branches
    branch1Label: isZh ? '自動回信' : 'Auto Reply',
    branch1Desc: isZh ? '依規則自動草擬並寄出回覆' : 'Draft and send replies per your rules',
    branch2Label: isZh ? '帳務整理' : 'Bill Accounting',
    branch2Desc: isZh ? '萃取帳單資訊，彙整月報表' : 'Extract billing info, aggregate reports',
    branch3Label: isZh ? '更多功能…' : 'More Features…',
    branch3Desc: isZh ? '投票敲碗，由社群決定下一步' : 'Vote below to shape what comes next',

    // Step-by-step explainer
    howTitle: isZh ? '三步驟快速上手' : 'Get Started in 3 Steps',
    how1Title: isZh ? '① 取得 AI 助理信箱地址' : '① Get Your AI Mailbox Address',
    how1Body: isZh
      ? '登入後前往「關於」頁面，複製專屬的 AI 助理電子郵件地址。'
      : 'After logging in, go to the About page and copy your dedicated AI assistant email address.',
    how2Title: isZh ? '② 在您的郵件服務設定轉寄規則' : '② Set Forwarding Rules in Your Email Provider',
    how2Body: isZh
      ? '在 Gmail、Outlook 或任何支援轉寄的郵件服務中，設定篩選條件（例如寄件者、主旨關鍵字），將符合條件的信件自動轉寄給 AI 助理信箱。'
      : 'In Gmail, Outlook, or any mail service that supports forwarding, add filters (e.g., sender, subject keyword) to auto-forward matching emails to your AI mailbox.',
    how3Title: isZh ? '③ 定義信件處理規則' : '③ Define Email Processing Rules',
    how3Body: isZh
      ? '在「規則」頁面或透過與 AI 助理對話，告訴助理如何處理每類信件——例如「收到帳單通知，自動歸檔到財務報表」。'
      : 'In the Rules page or via chat with the AI assistant, tell it how to handle each email type — e.g., "For billing notifications, auto-archive to finance report".',

    // Feature cards
    featuresTitle: isZh ? '目前支援的功能' : 'Currently Supported Features',
    autoReplyTitle: isZh ? '自動回信' : 'Auto Reply',
    autoReplyDesc: isZh
      ? '根據您定義的規則，AI 助理會分析來信內容，自動產生回覆草稿，並可設定為直接寄出。支援自訂助理名稱、語氣和回覆語言。'
      : 'Based on your defined rules, the AI assistant analyzes incoming email content, auto-generates reply drafts, and can be configured to send automatically. Supports custom assistant name, tone, and reply language.',
    billTitle: isZh ? '帳務整理' : 'Bill & Finance Accounting',
    billDesc: isZh
      ? '自動從信件中萃取帳單金額、繳費期限、發卡銀行、交易月份等財務資訊，彙整成月度報表，讓您掌握財務狀況一目了然。'
      : 'Automatically extract billing amounts, payment deadlines, card issuers, and transaction months from emails, aggregating them into monthly finance reports for a clear financial overview.',

    // Wish wall
    wishTitle: isZh ? '功能許願牆 — 投票敲碗 🗳️' : 'Feature Wish Wall — Vote & Request 🗳️',
    wishSubtitle: isZh
      ? '您想要哪些功能？對已有的建議投票，或送出您的新想法！票數越高，越優先開發。'
      : 'Which features do you want? Vote on existing suggestions or submit your own ideas! Higher votes = higher priority.',
    wishLoginPrompt: isZh
      ? '登入後即可投票及提交新功能建議'
      : 'Log in to vote and submit new feature suggestions',
    wishSubmitTitle: isZh ? '提交新功能建議' : 'Submit a New Feature Request',
    wishTitleLabel: isZh ? '功能名稱' : 'Feature Name',
    wishTitlePlaceholder: isZh ? '簡短描述您希望的功能（例如：智慧分類標籤）' : 'Briefly describe the feature (e.g., Smart Label Classification)',
    wishDescLabel: isZh ? '詳細說明（選填）' : 'Details (optional)',
    wishDescPlaceholder: isZh
      ? '可以說明使用情境、預期效果等'
      : 'Describe the use case, expected behavior, etc.',
    wishSubmitBtn: isZh ? '送出建議' : 'Submit Suggestion',
    votedBtnTip: isZh ? '取消我的投票' : 'Remove my vote',
    voteBtnTip: isZh ? '我要投這票' : 'Vote for this',
    officialBadge: isZh ? '官方功能' : 'Official',
    communityBadge: isZh ? '社群建議' : 'Community',
    noWishes: isZh ? '還沒有功能建議，快來第一個提交！' : 'No feature suggestions yet — be the first to submit!',
    voteCount: (n: number) => (isZh ? `${n} 票` : `${n} vote${n !== 1 ? 's' : ''}`),
    titleRequired: isZh ? '請輸入功能名稱' : 'Please enter a feature name',
    titleMaxLen: isZh ? '功能名稱最長 200 字' : 'Feature name must be ≤ 200 characters',
  };

  // ── Render ────────────────────────────────────────────────────────────────

  return (
    <div style={{ maxWidth: 900, margin: '0 auto', padding: '32px 16px' }}>

      {/* Page header */}
      <div style={{ textAlign: 'center', marginBottom: 48 }}>
        <Title level={2} style={{ marginBottom: 8 }}>
          {tx.pageTitle}
        </Title>
        <Paragraph type="secondary" style={{ fontSize: 15, maxWidth: 600, margin: '0 auto' }}>
          {tx.pageSubtitle}
        </Paragraph>
      </div>

      {/* ── Section A: Animated Flow Diagram ─────────────────────────────── */}
      <Card
        style={{ borderRadius: 20, marginBottom: 40, overflow: 'hidden' }}
        styles={{ body: { padding: '36px 24px' } }}
      >
        {/* Main pipeline (horizontal on wide, vertical on narrow) */}
        <div
          style={{
            display: 'flex',
            flexWrap: 'wrap',
            justifyContent: 'center',
            alignItems: 'center',
            gap: 0,
            marginBottom: 40,
          }}
        >
          <FlowNode
            icon={<MailOutlined />}
            label={tx.step1Label}
            sublabel={tx.step1Sub}
            delay={0}
            color="#0071e3"
          />
          <AnimatedArrow delay={0.2} />
          <FlowNode
            icon={<SettingOutlined />}
            label={tx.step2Label}
            sublabel={tx.step2Sub}
            delay={0.3}
            color="#34c759"
          />
          <AnimatedArrow delay={0.5} />
          <FlowNode
            icon={<RobotOutlined />}
            label={tx.step3Label}
            sublabel={tx.step3Sub}
            delay={0.6}
            color="#5856d6"
          />
          <AnimatedArrow delay={0.8} />
          <FlowNode
            icon={<ThunderboltOutlined />}
            label={tx.step4Label}
            sublabel={tx.step4Sub}
            delay={0.9}
            color="#ff9500"
          />
        </div>

        {/* Branch nodes */}
        <div
          style={{
            display: 'flex',
            flexWrap: 'wrap',
            justifyContent: 'center',
            gap: 16,
          }}
        >
          {/* Connector line from engine to branches */}
          <div style={{ width: '100%', display: 'flex', justifyContent: 'center', marginBottom: 4 }}>
            <svg width="400" height="32" style={{ overflow: 'visible' }}>
              {/* Horizontal bar */}
              <path
                className="flow-arrow-path"
                d="M60 4 L340 4"
                stroke="#ff9500"
                strokeWidth="2"
                fill="none"
                strokeLinecap="round"
              />
              {/* Three drops */}
              <line x1="100" y1="4" x2="100" y2="28" stroke="#ff9500" strokeWidth="2" strokeDasharray="4 2" opacity="0.7" />
              <line x1="200" y1="4" x2="200" y2="28" stroke="#ff9500" strokeWidth="2" strokeDasharray="4 2" opacity="0.7" />
              <line x1="300" y1="4" x2="300" y2="28" stroke="#8e8e93" strokeWidth="2" strokeDasharray="4 2" opacity="0.7" />
            </svg>
          </div>

          <BranchNode
            icon={<MailOutlined />}
            label={tx.branch1Label}
            desc={tx.branch1Desc}
            delay={1.1}
            color="#0071e3"
          />
          <BranchNode
            icon={<CrownOutlined />}
            label={tx.branch2Label}
            desc={tx.branch2Desc}
            delay={1.25}
            color="#34c759"
          />
          <BranchNode
            icon={<StarOutlined />}
            label={tx.branch3Label}
            desc={tx.branch3Desc}
            delay={1.4}
            isComingSoon
          />
        </div>
      </Card>

      {/* ── Section: Step-by-step explainer ─────────────────────────────── */}
      <Title level={4} style={{ marginBottom: 20 }}>
        {tx.howTitle}
      </Title>
      <Row gutter={[16, 16]} style={{ marginBottom: 40 }}>
        {[
          { icon: <RobotOutlined />, title: tx.how1Title, body: tx.how1Body, color: '#5856d6' },
          { icon: <SettingOutlined />, title: tx.how2Title, body: tx.how2Body, color: '#34c759' },
          { icon: <BulbOutlined />, title: tx.how3Title, body: tx.how3Body, color: '#ff9500' },
        ].map((step, i) => (
          <Col xs={24} md={8} key={i}>
            <Card
              style={{ borderRadius: 16, height: '100%' }}
              styles={{ body: { padding: '20px 18px' } }}
            >
              <div style={{ fontSize: 28, color: step.color, marginBottom: 10 }}>{step.icon}</div>
              <Text strong style={{ display: 'block', marginBottom: 8, fontSize: 14 }}>
                {step.title}
              </Text>
              <Paragraph type="secondary" style={{ fontSize: 13, marginBottom: 0 }}>
                {step.body}
              </Paragraph>
            </Card>
          </Col>
        ))}
      </Row>

      {/* ── Section B: Supported feature cards ───────────────────────────── */}
      <Title level={4} style={{ marginBottom: 20 }}>
        {tx.featuresTitle}
      </Title>
      <Row gutter={[16, 16]} style={{ marginBottom: 48 }}>
        <Col xs={24} md={12}>
          <Card
            style={{ borderRadius: 16, border: '1.5px solid #0071e3' }}
            styles={{ body: { padding: '22px 20px' } }}
          >
            <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 12 }}>
              <div
                style={{
                  width: 42,
                  height: 42,
                  borderRadius: 12,
                  background: 'linear-gradient(135deg, #0071e322, #0071e344)',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  fontSize: 20,
                  color: '#0071e3',
                }}
              >
                <MailOutlined />
              </div>
              <Title level={5} style={{ margin: 0 }}>
                {tx.autoReplyTitle}
              </Title>
              <Tag color="blue" style={{ marginLeft: 'auto' }}>
                <CheckCircleFilled style={{ marginRight: 4 }} />
                {isZh ? '已上線' : 'Live'}
              </Tag>
            </div>
            <Paragraph type="secondary" style={{ fontSize: 13, marginBottom: 0 }}>
              {tx.autoReplyDesc}
            </Paragraph>
          </Card>
        </Col>
        <Col xs={24} md={12}>
          <Card
            style={{ borderRadius: 16, border: '1.5px solid #34c759' }}
            styles={{ body: { padding: '22px 20px' } }}
          >
            <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 12 }}>
              <div
                style={{
                  width: 42,
                  height: 42,
                  borderRadius: 12,
                  background: 'linear-gradient(135deg, #34c75922, #34c75944)',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  fontSize: 20,
                  color: '#34c759',
                }}
              >
                <CrownOutlined />
              </div>
              <Title level={5} style={{ margin: 0 }}>
                {tx.billTitle}
              </Title>
              <Tag color="green" style={{ marginLeft: 'auto' }}>
                <CheckCircleFilled style={{ marginRight: 4 }} />
                {isZh ? '已上線' : 'Live'}
              </Tag>
            </div>
            <Paragraph type="secondary" style={{ fontSize: 13, marginBottom: 0 }}>
              {tx.billDesc}
            </Paragraph>
          </Card>
        </Col>
      </Row>

      {/* ── Section C: Wish Wall ──────────────────────────────────────────── */}
      <div style={{ marginBottom: 12 }}>
        <Title level={4} style={{ marginBottom: 4 }}>
          {tx.wishTitle}
        </Title>
        <Paragraph type="secondary" style={{ marginBottom: 24 }}>
          {tx.wishSubtitle}
        </Paragraph>
      </div>

      {/* Submit form — only for logged-in users */}
      {user ? (
        <Card
          style={{ borderRadius: 16, marginBottom: 24, background: '#f0f6ff' }}
          styles={{ body: { padding: '20px 20px' } }}
        >
          <Title level={5} style={{ marginBottom: 16 }}>
            <PlusOutlined style={{ marginRight: 8, color: '#0071e3' }} />
            {tx.wishSubmitTitle}
          </Title>
          <Form form={form} layout="vertical" onFinish={handleSubmitWish}>
            <Form.Item
              name="title"
              label={tx.wishTitleLabel}
              rules={[
                { required: true, message: tx.titleRequired },
                { max: 200, message: tx.titleMaxLen },
              ]}
            >
              <Input placeholder={tx.wishTitlePlaceholder} maxLength={200} />
            </Form.Item>
            <Form.Item name="description" label={tx.wishDescLabel}>
              <TextArea
                placeholder={tx.wishDescPlaceholder}
                rows={3}
                maxLength={1000}
                showCount
              />
            </Form.Item>
            <Form.Item style={{ marginBottom: 0 }}>
              <Button
                type="primary"
                htmlType="submit"
                loading={submitting}
                icon={<PlusOutlined />}
              >
                {tx.wishSubmitBtn}
              </Button>
            </Form.Item>
          </Form>
        </Card>
      ) : (
        <Alert
          icon={<LockOutlined />}
          showIcon
          type="info"
          message={tx.wishLoginPrompt}
          style={{ borderRadius: 12, marginBottom: 24 }}
        />
      )}

      {/* Wishes list */}
      {wishesLoading ? (
        <div style={{ textAlign: 'center', padding: '40px 0' }}>
          <Spin size="large" />
        </div>
      ) : (
        <List
          dataSource={wishes}
          locale={{ emptyText: tx.noWishes }}
          renderItem={(wish) => {
            const isVoting = votingId === wish.id;
            return (
              <List.Item
                key={wish.id}
                style={{
                  background: '#fff',
                  borderRadius: 14,
                  padding: '16px 20px',
                  marginBottom: 10,
                  border: wish.is_official ? '1.5px solid #0071e333' : '1px solid #e8e8e8',
                }}
                actions={[
                  <Tooltip
                    key="vote"
                    title={wish.user_has_voted ? tx.votedBtnTip : tx.voteBtnTip}
                  >
                    <Button
                      type={wish.user_has_voted ? 'primary' : 'default'}
                      icon={wish.user_has_voted ? <LikeFilled /> : <LikeOutlined />}
                      loading={isVoting}
                      onClick={() => handleVote(wish)}
                      style={{ borderRadius: 20, minWidth: 72 }}
                    >
                      {tx.voteCount(wish.vote_count)}
                    </Button>
                  </Tooltip>,
                ]}
              >
                <List.Item.Meta
                  title={
                    <span style={{ display: 'flex', alignItems: 'center', gap: 8, flexWrap: 'wrap' }}>
                      <Text strong style={{ fontSize: 14 }}>
                        {wish.title}
                      </Text>
                      {wish.is_official ? (
                        <Badge
                          count={tx.officialBadge}
                          style={{ backgroundColor: '#0071e3', fontSize: 11, borderRadius: 6 }}
                        />
                      ) : (
                        <Badge
                          count={tx.communityBadge}
                          style={{ backgroundColor: '#8e8e93', fontSize: 11, borderRadius: 6 }}
                        />
                      )}
                    </span>
                  }
                  description={
                    wish.description ? (
                      <Text type="secondary" style={{ fontSize: 13 }}>
                        {wish.description}
                      </Text>
                    ) : undefined
                  }
                />
              </List.Item>
            );
          }}
        />
      )}
    </div>
  );
};

export default HowItWorksPage;
