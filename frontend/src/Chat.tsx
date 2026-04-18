import React, { useState, useEffect, useRef, useCallback } from 'react';
import { Card, Input, Button, List, Avatar, Tag, Modal, Space } from 'antd';
import { SendOutlined, RobotOutlined, UserOutlined, EditOutlined } from '@ant-design/icons';
import axios from 'axios';
import { useAuth } from './AuthContext';



interface Message {
  sender: 'ai' | 'user';
  text: string;
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

function loadMessages(email: string | undefined): Message[] {
  try {
    const raw = localStorage.getItem(getStorageKey(email));
    if (raw) return JSON.parse(raw) as Message[];
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
        setMessages([{ sender: 'ai', text: `Hello ${greetingName} I am your AI Mail Butler assistant. Feel free to ask me anything about email management, or login to unlock personalised features.` }]);
      } else if (!user.is_onboarded) {
        setMessages([{ sender: 'ai', text: `Welcome ${name || user.email}! I am your AI Mail Butler. I noticed you are new here. What kind of emails do you usually handle, and how would you like me to process them?` }]);
      } else {
        setMessages([{ sender: 'ai', text: `Welcome back ${name || user.email}! Your current preferences are: ${user.preferences || 'None'}. How can I assist you today?` }]);
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
    setMessages(prev => [...prev, { sender: 'user', text: userMsg }]);
    setInput('');
    setLoading(true);

    try {
      // Pass guest_name to backend if user is anonymous (we can extend the API to accept it)
      const res = await axios.post('/api/chat', { 
        email: user?.email ?? '', 
        message: userMsg,
        guest_name: !user ? guestName : undefined
      });
      setMessages(prev => [...prev, { sender: 'ai', text: res.data.reply }]);
      if (user && !user.is_onboarded && res.data.reply.toLowerCase().includes('noted')) {
        refreshUser();
      }
    } catch {
      setMessages(prev => [...prev, { sender: 'ai', text: 'Sorry, I encountered an error communicating with the server.' }]);
    } finally {
      setLoading(false);
    }
  }, [input, user, guestName, refreshUser]);

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
          renderItem={(msg) => (
            <List.Item style={{ borderBottom: 'none', justifyContent: msg.sender === 'user' ? 'flex-end' : 'flex-start' }}>
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
                <div style={{
                  padding: '12px 16px',
                  background: msg.sender === 'user' ? '#0071e3' : '#f5f5f7',
                  color: msg.sender === 'user' ? '#fff' : '#1d1d1f',
                  borderRadius: 16,
                  borderTopRightRadius: msg.sender === 'user' ? 4 : 16,
                  borderTopLeftRadius: msg.sender === 'ai' ? 4 : 16,
                  maxWidth: '70vw',
                }}>
                  <div style={{ fontSize: '14px' }}>
                    <MessageContent text={msg.text} />
                  </div>
                </div>
              </div>
            </List.Item>
          )}
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
    </Card>
  );
};
