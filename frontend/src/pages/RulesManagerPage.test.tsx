import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import axios from 'axios'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useAuth } from '../AuthContext'
import { makeUser } from '../test/factories'
import RulesManagerPage from './RulesManagerPage'

vi.mock('axios', () => ({
  default: {
    get: vi.fn(),
    post: vi.fn(),
  },
}))

vi.mock('../AuthContext', () => ({
  useAuth: vi.fn(),
}))

const mockedUseAuth = vi.mocked(useAuth)
const mockedGet = vi.mocked(axios.get)
const mockedPost = vi.mocked(axios.post)

describe('RulesManagerPage', () => {
  beforeEach(() => {
    mockedUseAuth.mockReset()
    mockedGet.mockReset()
    mockedPost.mockReset()
  })

  it('shows login guidance for guests', () => {
    mockedUseAuth.mockReturnValue({
      user: null,
      refreshUser: vi.fn(),
      requestMagicLink: vi.fn(),
      verifyToken: vi.fn(),
      logout: vi.fn(),
      loading: false,
      api: '/api',
    })

    render(<RulesManagerPage />)

    expect(screen.getByText('Please login first')).toBeInTheDocument()
    expect(mockedGet).not.toHaveBeenCalled()
  })

  it('loads rules and creates a new manual rule', async () => {
    mockedUseAuth.mockReturnValue({
      user: makeUser({ email: 'rules@example.com' }),
      refreshUser: vi.fn(),
      requestMagicLink: vi.fn(),
      verifyToken: vi.fn(),
      logout: vi.fn(),
      loading: false,
      api: '/api',
    })
    mockedGet.mockResolvedValue({
      data: {
        rules: [
          {
            id: 7,
            rule_text: 'If invoice arrives, summarize it',
            rule_label: 'RULE-INVOICE',
            source: 'manual',
            is_enabled: true,
            matched_count: 2,
            updated_at: '2026-05-13 09:00:00',
          },
        ],
      },
    })
    mockedPost.mockResolvedValue({ data: { status: 'success' } })

    render(<RulesManagerPage />)

    expect(await screen.findByText('If invoice arrives, summarize it')).toBeInTheDocument()
    expect(screen.getByText('INVOICE')).toBeInTheDocument()

    await userEvent.type(
      screen.getByPlaceholderText('Example: 如果是信用卡帳單，先摘要重點再提醒繳費期限'),
      'Forward receipts to finance',
    )
    await userEvent.click(screen.getByRole('button', { name: /add rule/i }))

    await waitFor(() => {
      expect(mockedPost).toHaveBeenCalledWith('/api/rules/create', {
        email: 'rules@example.com',
        rule_text: 'Forward receipts to finance',
      })
    })
  })

  it('toggles and deletes existing rules', async () => {
    mockedUseAuth.mockReturnValue({
      user: makeUser({ email: 'rules@example.com' }),
      refreshUser: vi.fn(),
      requestMagicLink: vi.fn(),
      verifyToken: vi.fn(),
      logout: vi.fn(),
      loading: false,
      api: '/api',
    })
    mockedGet.mockResolvedValue({
      data: {
        rules: [
          {
            id: 8,
            rule_text: 'Archive newsletters',
            rule_label: '',
            source: 'chat',
            is_enabled: true,
            matched_count: 0,
            updated_at: '',
          },
        ],
      },
    })
    mockedPost.mockResolvedValue({ data: { status: 'success' } })

    render(<RulesManagerPage />)

    await waitFor(() => {
      expect(screen.getAllByText('Archive newsletters').length).toBeGreaterThan(0)
    })

    await userEvent.click(screen.getByRole('switch'))
    await waitFor(() => {
      expect(mockedPost).toHaveBeenCalledWith('/api/rules/toggle', {
        email: 'rules@example.com',
        id: 8,
        is_enabled: false,
      })
    })

    await userEvent.click(screen.getByRole('button', { name: /delete/i }))
    await userEvent.click(screen.getByRole('button', { name: /^yes$/i }))

    await waitFor(() => {
      expect(mockedPost).toHaveBeenCalledWith('/api/rules/delete', {
        email: 'rules@example.com',
        id: 8,
      })
    })
  })
})
