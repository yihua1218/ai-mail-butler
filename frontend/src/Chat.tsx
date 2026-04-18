import React, { useState, useEffect, useRef } from 'react';
import { Card, Input, Button, List, Typography, Avatar } from 'antd';
import { SendOutlined, RobotOutlined, UserOutlined } from '@ant-design/icons';
import axios from 'axios';
import { useAuth } from './AuthContext';

const { Text } = Typography;

interface Message {
  sender: 'ai' | 'user';
  text: string;
}

export const Chat: React.FC = () => {
  const { user, refreshUser } = useAuth();
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState('');
  const [loading, setLoading] = useState(false);
  const listRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (user && messages.length === 0) {
      if (!user.is_onboarded) {
        setMessages([{ sender: 'ai', text: `Welcome ${user.email}! I am your AI Mail Butler. I noticed you are new here. What kind of emails do you usually handle, and how would you like me to process them (e.g., summarize, translate, auto-reply)?` }]);
      } else {
        setMessages([{ sender: 'ai', text: `Welcome back! Your current preferences are: ${user.preferences || 'None'}. How can I assist you today?` }]);
      }
    }
  }, [user]);

  useEffect(() => {
    if (listRef.current) {
      listRef.current.scrollTop = listRef.current.scrollHeight;
    }
  }, [messages]);

  const handleSend = async () => {
    if (!input.trim() || !user) return;

    const userMsg = input.trim();
    setMessages(prev => [...prev, { sender: 'user', text: userMsg }]);
    setInput('');
    setLoading(true);

    try {
      const res = await axios.post('/api/chat', { email: user.email, message: userMsg });
      setMessages(prev => [...prev, { sender: 'ai', text: res.data.reply }]);
      // Refresh user context implicitly or explicitly if needed
      if (!user.is_onboarded && res.data.reply.includes("noted down")) {
        refreshUser(); // Re-fetch to update onboarded status
      }
    } catch (e) {
      setMessages(prev => [...prev, { sender: 'ai', text: 'Sorry, I encountered an error communicating with the server.' }]);
    } finally {
      setLoading(false);
    }
  };

  if (!user) {
    return <Card style={{ textAlign: 'center', marginTop: 40 }}><Text>Please login to use the AI Assistant.</Text></Card>;
  }

  return (
    <Card 
      title="AI Mail Butler Assistant" 
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
                maxWidth: '80%'
              }}>
                <Avatar 
                  icon={msg.sender === 'ai' ? <RobotOutlined /> : <UserOutlined />} 
                  style={{ 
                    backgroundColor: msg.sender === 'ai' ? '#0071e3' : '#34c759',
                    margin: msg.sender === 'user' ? '0 0 0 12px' : '0 12px 0 0'
                  }} 
                />
                <div style={{
                  padding: '12px 16px',
                  background: msg.sender === 'user' ? '#0071e3' : '#f5f5f7',
                  color: msg.sender === 'user' ? '#fff' : '#1d1d1f',
                  borderRadius: 16,
                  borderTopRightRadius: msg.sender === 'user' ? 4 : 16,
                  borderTopLeftRadius: msg.sender === 'ai' ? 4 : 16,
                }}>
                  <Text style={{ color: 'inherit' }}>{msg.text}</Text>
                </div>
              </div>
            </List.Item>
          )}
        />
      </div>
      <div style={{ padding: '20px', borderTop: '1px solid #f0f0f0' }}>
        <Input.Search
          placeholder="Type your message..."
          enterButton={<Button type="primary" icon={<SendOutlined />} loading={loading} />}
          size="large"
          value={input}
          onChange={e => setInput(e.target.value)}
          onSearch={handleSend}
          disabled={loading}
        />
      </div>
    </Card>
  );
};
