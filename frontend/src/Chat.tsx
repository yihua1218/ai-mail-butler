import React, { useState, useEffect, useRef, useCallback } from 'react';
import { Card, Input, Button, List, Avatar, Tag, Modal, Space, Tooltip, message as antdMessage } from 'antd';
import { 
  SendOutlined, RobotOutlined, UserOutlined, EditOutlined, 
  BulbOutlined, DashboardOutlined, FieldBinaryOutlined, ClockCircleOutlined,
  LikeOutlined, DislikeOutlined
} from '@ant-design/icons';
import axios from 'axios';
import { useAuth } from './AuthContext';

interface Message {
  id: string;
  sender: 'ai' | 'user';
  text: string;
  timestamp?: string;
  tokens?: number;
  duration_ms?: number;
  finish_reason?: string;
  feedback?: 'up' | 'down';
  feedback_submitted?: boolean;
  debug?: any; // raw API response for debugging
}

// ---------------------------------------------------------------------------
// Gravatar helpers (SHA-256 supported by Gravatar since 2024)
// ---------------------------------------------------------------------------
async function getGravatarUrl(email: string): Promise<string> {
  const normalised = email.trim().toLowerCase();
  const data = new TextEncoder().encode(normalised);
  const hashBuffer = await crypto.subtle.digest('SHA-256', data);
  const hashHex = Array.from(new Uint8Array(hashBuffer))
    .map(b => b.toString(16).padStart(2, '0'))
    .join('');
  return `https://www.gravatar.com/avatar/${hashHex}?s=64&d=404`;
}

async function resolveAvatarUrl(email: string): Promise<string | null> {
  try {
    const url = await getGravatarUrl(email);
    const res = await fetch(url, { method: 'HEAD' });
    return res.ok ? url.replace('d=404', 'd=mp') : null;
  } catch {
    return null;
  }
}

// ---------------------------------------------------------------------------
// Improved Markdown renderer (bold, italic, bullet points)
// ---------------------------------------------------------------------------
function renderInlineMarkdown(text: string): React.ReactNode[] {
  const parts = text.split(/(\*\*[^*]+\*\*|\*[^*]+\*)/g);
  return parts.map((part, i) => {
    if (part.startsWith('**') && part.endsWith('**')) {
      return <strong key={i}>{part.slice(2, -2)}</strong>;
    }
    if (part.startsWith('*') && part.endsWith('*')) {
      return <em key={i}>{part.slice(1, -1)}</em>;
    }
    return <span key={i}>{part}</span>;
  });
}

function MessageContent({ text }: { text: string }) {
  const lines = text.split('\n');
  const renderedElements: React.ReactNode[] = [];
  let currentListItems: React.ReactNode[] = [];

  const flushList = () => {
    if (currentListItems.length > 0) {
      renderedElements.push(
        <ul key={`list-${renderedElements.length}`} style={{ paddingLeft: '20px', margin: '8px 0' }}>
          {currentListItems}
        </ul>
      );
      currentListItems = [];
    }
  };

  lines.forEach((line, index) => {
    const trimmed = line.trim();
    if (trimmed.startsWith('* ') || trimmed.startsWith('- ')) {
      currentListItems.push(
        <li key={`li-${index}`} style={{ marginBottom: '4px' }}>
          {renderInlineMarkdown(trimmed.substring(2))}
        </li>
      );
    } else if (trimmed === '') {
      flushList();
      renderedElements.push(<div key={`br-${index}`} style={{ height: '8px' }} />);
    } else {
      flushList();
      renderedElements.push(
        <div key={`line-${index}`} style={{ marginBottom: '4px' }}>
          {renderInlineMarkdown(line)}
        </div>
      );
    }
  });
  flushList();

  return <>{renderedElements}</>;
}

// ---------------------------------------------------------------------------
// localStorage persistence helpers
// ---------------------------------------------------------------------------
const STORAGE_KEY_PREFIX = 'ai_mail_butler_chat_';
export const GUEST_NAME_KEY = 'ai_mail_butler_guest_name';

function getStorageKey(email: string | undefined): string {
  return STORAGE_KEY_PREFIX + (email ?? 'anonymous');
}

function createMessageId(): string {
  return `${Date.now()}-${Math.random().toString(36).slice(2, 10)}`;
}

function loadMessages(email: string | undefined): Message[] {
  try {
    const raw = localStorage.getItem(getStorageKey(email));
    if (raw) {
      const parsed = JSON.parse(raw) as Message[];
      return parsed.map((m) => ({ ...m, id: m.id || createMessageId() }));
    }
  } catch { /* ignore */ }
  return [];
}

function saveMessages(email: string | undefined, messages: Message[]): void {
  try {
    localStorage.setItem(getStorageKey(email), JSON.stringify(messages));
  } catch { /* ignore */ }
}

// ---------------------------------------------------------------------------
// Chat component
// ---------------------------------------------------------------------------
export const Chat: React.FC = () => {
  const { user, refreshUser } = useAuth();
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState('');
  const [loading, setLoading] = useState(false);
  const [avatarUrl, setAvatarUrl] = useState<string | null>(null);
  const [guestName, setGuestName] = useState<string>(() => localStorage.getItem(GUEST_NAME_KEY) || '');
  const [isNameModalVisible, setIsNameModalVisible] = useState(false);
  const [tempName, setTempName] = useState(guestName);
  const [feedbackModalOpen, setFeedbackModalOpen] = useState(false);
  const [feedbackMessageId, setFeedbackMessageId] = useState<string | null>(null);
  const [feedbackSuggestion, setFeedbackSuggestion] = useState('');
  const [submittingFeedback, setSubmittingFeedback] = useState(false);
  const listRef = useRef<HTMLDivElement>(null);

  // Load messages from localStorage when user identity changes
  useEffect(() => {
    const saved = loadMessages(user?.email);
    if (saved.length > 0) {
      setMessages(saved);
    } else {
      const name = user?.display_name || guestName;
      const greetingName = name ? `${name}!` : 'there!';
      if (!user) {
        setMessages([{ id: createMessageId(), sender: 'ai', text: `Hello ${greetingName} I am your AI Mail Butler assistant. Feel free to ask me anything about email management, or login to unlock personalised features.`, timestamp: new Date().toISOString() }]);
      } else if (!user.is_onboarded) {
        setMessages([{ id: createMessageId(), sender: 'ai', text: `Welcome ${name || user.email}! I am your AI Mail Butler. I noticed you are new here. What kind of emails do you usually handle, and how would you like me to process them?`, timestamp: new Date().toISOString() }]);
      } else {
        setMessages([{ id: createMessageId(), sender: 'ai', text: `Welcome back ${name || user.email}! Your current preferences are: ${user.preferences || 'None'}. How can I assist you today?`, timestamp: new Date().toISOString() }]);
      }
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [user?.email, user?.display_name]);

  // Resolve Gravatar when user email changes
  useEffect(() => {
    setAvatarUrl(null);
    if (user?.email) {
      resolveAvatarUrl(user.email).then(url => setAvatarUrl(url));
    }
  }, [user?.email]);

  // Persist messages on every change
  useEffect(() => {
    if (messages.length > 0) {
      saveMessages(user?.email, messages);
    }
  }, [messages, user?.email]);

  // Auto-scroll to bottom
  useEffect(() => {
    if (listRef.current) {
      listRef.current.scrollTop = listRef.current.scrollHeight;
    }
  }, [messages]);

  const handleSend = useCallback(async () => {
    if (!input.trim()) return;

    const userMsg = input.trim();
    const now = new Date().toISOString();
    setMessages(prev => [...prev, { id: createMessageId(), sender: 'user', text: userMsg, timestamp: now }]);
    setInput('');
    setLoading(true);

    try {
      const res = await axios.post('/api/chat', { 
        email: user?.email ?? '', 
        message: userMsg,
        guest_name: !user ? guestName : undefined
      });
      
      const aiMsg: Message = { 
        id: createMessageId(),
        sender: 'ai', 
        text: res.data.reply,
        timestamp: res.data.timestamp || new Date().toISOString(),
        tokens: res.data.total_tokens,
        duration_ms: res.data.duration_ms,
        finish_reason: res.data.finish_reason,
        debug: res.data
      };

      setMessages(prev => [...prev, aiMsg]);
      if (user && !user.is_onboarded && res.data.reply.toLowerCase().includes('noted')) {
        refreshUser();
      }
    } catch {
      setMessages(prev => [...prev, { id: createMessageId(), sender: 'ai', text: 'Sorry, I encountered an error communicating with the server.', timestamp: new Date().toISOString() }]);
    } finally {
      setLoading(false);
    }
  }, [input, user, guestName, refreshUser]);

  const submitFeedback = useCallback(async (target: Message, rating: 'up' | 'down', suggestion?: string) => {
    try {
      await axios.post('/api/chat/feedback', {
        email: user?.email,
        ai_reply: target.text,
        rating,
        suggestion,
      });
      setMessages((prev) =>
        prev.map((m) =>
          m.id === target.id ? { ...m, feedback: rating, feedback_submitted: true } : m
        )
      );
      antdMessage.success(rating === 'up' ? '感謝你的肯定回饋！' : '已收到，我會持續改進。');
    } catch {
      antdMessage.error('回饋送出失敗，請稍後再試。');
    }
  }, [user?.email]);

  const onFeedbackClick = useCallback((target: Message, rating: 'up' | 'down') => {
    if (target.feedback_submitted) return;
    if (rating === 'up') {
      submitFeedback(target, 'up');
      return;
    }

    setFeedbackMessageId(target.id);
    setFeedbackSuggestion('');
    setFeedbackModalOpen(true);
  }, [submitFeedback]);

  const handleSubmitDownFeedback = useCallback(async () => {
    const target = messages.find((m) => m.id === feedbackMessageId && m.sender === 'ai');
    if (!target) {
      setFeedbackModalOpen(false);
      return;
    }
    setSubmittingFeedback(true);
    await submitFeedback(target, 'down', feedbackSuggestion.trim() || undefined);
    setSubmittingFeedback(false);
    setFeedbackModalOpen(false);
    setFeedbackMessageId(null);
    setFeedbackSuggestion('');
  }, [feedbackMessageId, feedbackSuggestion, messages, submitFeedback]);

  const handleSkipSuggestion = useCallback(async () => {
    const target = messages.find((m) => m.id === feedbackMessageId && m.sender === 'ai');
    if (!target) {
      setFeedbackModalOpen(false);
      return;
    }
    setSubmittingFeedback(true);
    await submitFeedback(target, 'down');
    setSubmittingFeedback(false);
    setFeedbackModalOpen(false);
    setFeedbackMessageId(null);
    setFeedbackSuggestion('');
  }, [feedbackMessageId, messages, submitFeedback]);

  const saveGuestName = () => {
    localStorage.setItem(GUEST_NAME_KEY, tempName);
    setGuestName(tempName);
    setIsNameModalVisible(false);
  };

  const cardTitle = (
    <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', width: '100%' }}>
      <Space>
        <span>AI Mail Butler Assistant</span>
        {!user && <Tag color="default">Guest Mode</Tag>}
      </Space>
      {!user && (
        <Button 
          type="text" 
          size="small" 
          icon={<EditOutlined />} 
          onClick={() => { setTempName(guestName); setIsNameModalVisible(true); }}
        >
          {guestName ? `Hi, ${guestName}` : 'Set Nickname'}
        </Button>
      )}
    </div>
  );

  return (
    <Card
      title={cardTitle}
      bordered={false}
      style={{ height: '70vh', display: 'flex', flexDirection: 'column' }}
      styles={{ body: { flex: 1, display: 'flex', flexDirection: 'column', padding: 0, overflow: 'hidden' } }}
    >
      <div ref={listRef} style={{ flex: 1, overflowY: 'auto', padding: '20px' }}>
        <List
          itemLayout="horizontal"
          dataSource={messages}
          renderItem={(msg) => {
            const timeStr = msg.timestamp ? new Date(msg.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', hour12: false }) : '';
            const speed = (msg.tokens && msg.duration_ms) ? (msg.tokens / (msg.duration_ms / 1000)).toFixed(1) : null;
            const durationSec = msg.duration_ms ? (msg.duration_ms / 1000).toFixed(2) : null;

            return (
              <List.Item style={{ borderBottom: 'none', justifyContent: msg.sender === 'user' ? 'flex-end' : 'flex-start', padding: '12px 0' }}>
                <div style={{
                  display: 'flex',
                  flexDirection: msg.sender === 'user' ? 'row-reverse' : 'row',
                  alignItems: 'flex-start',
                }}>
                  {msg.sender === 'ai' ? (
                    <Avatar
                      icon={<RobotOutlined />}
                      style={{ backgroundColor: '#0071e3', flexShrink: 0, marginRight: 12 }}
                    />
                  ) : avatarUrl ? (
                    <Avatar
                      src={avatarUrl}
                      style={{ flexShrink: 0, marginLeft: 12 }}
                    />
                  ) : (
                    <Avatar
                      icon={<UserOutlined />}
                      style={{ backgroundColor: '#34c759', flexShrink: 0, marginLeft: 12 }}
                    />
                  )}
                  <div style={{ display: 'flex', flexDirection: 'column', alignItems: msg.sender === 'user' ? 'flex-end' : 'flex-start' }}>
                    <div style={{
                      padding: '12px 16px',
                      background: msg.sender === 'user' ? '#0071e3' : '#f5f5f7',
                      color: msg.sender === 'user' ? '#fff' : '#1d1d1f',
                      borderRadius: 18,
                      borderTopRightRadius: msg.sender === 'user' ? 4 : 18,
                      borderTopLeftRadius: msg.sender === 'ai' ? 4 : 18,
                      maxWidth: '70vw',
                      boxShadow: '0 2px 8px rgba(0,0,0,0.04)',
                    }}>
                      <div style={{ fontSize: '15px', lineHeight: '1.5' }}>
                        <MessageContent text={msg.text} />
                      </div>
                    </div>
                    
                    <div style={{ 
                      marginTop: 8, 
                      display: 'flex', 
                      flexDirection: 'column',
                      alignItems: msg.sender === 'user' ? 'flex-end' : 'flex-start'
                    }}>
                      <div style={{ 
                        display: 'flex', 
                        alignItems: 'center',
                        gap: '6px',
                        flexWrap: 'wrap'
                      }}>
                        {msg.sender === 'ai' && msg.debug && (
                          <Tooltip 
                            title={<pre style={{ margin: 0, fontSize: '11px', maxHeight: '400px', overflow: 'auto' }}>{JSON.stringify(msg.debug, null, 2)}</pre>} 
                            placement="bottomLeft"
                            overlayStyle={{ maxWidth: '600px' }}
                          >
                            <BulbOutlined style={{ color: '#86868b', fontSize: '14px', cursor: 'help', marginRight: 4 }} />
                          </Tooltip>
                        )}

                        <span style={{ fontSize: '11px', color: '#86868b', marginRight: 4 }}>{timeStr}</span>

                        {msg.sender === 'ai' && (
                          <>
                            {speed && (
                              <Tag bordered={false} icon={<DashboardOutlined />} style={{ borderRadius: 12, margin: 0, fontSize: '11px', background: '#f5f5f7', color: '#424245' }}>
                                {speed} tok/sec
                              </Tag>
                            )}
                            {msg.tokens !== undefined && msg.tokens > 0 && (
                              <Tag bordered={false} icon={<FieldBinaryOutlined />} style={{ borderRadius: 12, margin: 0, fontSize: '11px', background: '#f5f5f7', color: '#424245' }}>
                                {msg.tokens} tokens
                              </Tag>
                            )}
                            {durationSec && (
                              <Tag bordered={false} icon={<ClockCircleOutlined />} style={{ borderRadius: 12, margin: 0, fontSize: '11px', background: '#f5f5f7', color: '#424245' }}>
                                {durationSec}s
                              </Tag>
                            )}
                            {msg.finish_reason && (
                              <Tag bordered={false} style={{ borderRadius: 12, margin: 0, fontSize: '11px', background: '#f5f5f7', color: '#424245' }}>
                                Stop reason: {msg.finish_reason}
                              </Tag>
                            )}
                            <Space size={4}>
                              <Button
                                type={msg.feedback === 'up' ? 'primary' : 'text'}
                                size="small"
                                icon={<LikeOutlined />}
                                disabled={!!msg.feedback_submitted}
                                onClick={() => onFeedbackClick(msg, 'up')}
                              />
                              <Button
                                type={msg.feedback === 'down' ? 'primary' : 'text'}
                                danger={msg.feedback === 'down'}
                                size="small"
                                icon={<DislikeOutlined />}
                                disabled={!!msg.feedback_submitted}
                                onClick={() => onFeedbackClick(msg, 'down')}
                              />
                            </Space>
                          </>
                        )}
                      </div>
                    </div>
                  </div>
                </div>
              </List.Item>
            );
          }}
        />
      </div>
      <div style={{ padding: '20px', borderTop: '1px solid #f0f0f0' }}>
        <Input.Search
          placeholder={loading ? "Waiting for response..." : "Type your message..."}
          enterButton={<Button type="primary" icon={<SendOutlined />} loading={loading} />}
          size="large"
          value={input}
          onChange={e => setInput(e.target.value)}
          onSearch={handleSend}
          disabled={loading}
        />
      </div>

      <Modal
        title="What should I call you?"
        open={isNameModalVisible}
        onOk={saveGuestName}
        onCancel={() => setIsNameModalVisible(false)}
        okText="Save"
      >
        <Input 
          placeholder="Your nickname" 
          value={tempName} 
          onChange={e => setTempName(e.target.value)}
          onPressEnter={saveGuestName}
          autoFocus
        />
        <p style={{ marginTop: 12, color: '#86868b', fontSize: '12px' }}>
          This will be stored in your browser so I can address you by name.
        </p>
      </Modal>

      <Modal
        title="這則回覆不理想嗎？"
        open={feedbackModalOpen}
        onCancel={handleSkipSuggestion}
        footer={[
          <Button key="skip" onClick={handleSkipSuggestion} disabled={submittingFeedback}>
            略過建議，直接送出 👎
          </Button>,
          <Button
            key="submit"
            type="primary"
            loading={submittingFeedback}
            onClick={handleSubmitDownFeedback}
          >
            送出改善建議
          </Button>
        ]}
      >
        <p style={{ marginBottom: 10 }}>是否要補充你期待的回答方向？</p>
        <Input.TextArea
          rows={4}
          value={feedbackSuggestion}
          onChange={(e) => setFeedbackSuggestion(e.target.value)}
          placeholder="例如：希望更精簡、要先列重點、語氣改成更正式..."
        />
      </Modal>
    </Card>
  );
};
