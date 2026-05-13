import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import axios from 'axios'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useAuth } from '../AuthContext'
import { GUEST_NAME_KEY } from '../Chat'
import { makeUser } from '../test/factories'
import SettingsPage from './SettingsPage'

vi.mock('axios', () => ({
  default: {
    post: vi.fn(),
  },
}))

vi.mock('../AuthContext', () => ({
  useAuth: vi.fn(),
}))

const mockedUseAuth = vi.mocked(useAuth)
const mockedPost = vi.mocked(axios.post)

describe('SettingsPage', () => {
  beforeEach(() => {
    localStorage.clear()
    mockedPost.mockReset()
    mockedUseAuth.mockReset()
  })

  it('saves guest display name locally without calling backend settings API', async () => {
    mockedUseAuth.mockReturnValue({
      user: null,
      refreshUser: vi.fn(),
      requestMagicLink: vi.fn(),
      verifyToken: vi.fn(),
      logout: vi.fn(),
      loading: false,
      api: '/api',
    })

    render(<SettingsPage />)

    await userEvent.type(screen.getByPlaceholderText('e.g. Yihua, Master, etc.'), 'Guest Tester')
    await userEvent.click(screen.getByRole('button', { name: /save settings/i }))

    expect(localStorage.getItem(GUEST_NAME_KEY)).toBe('Guest Tester')
    expect(mockedPost).not.toHaveBeenCalledWith('/api/settings', expect.anything())
  })

  it('submits logged-in settings and refreshes the current user', async () => {
    const refreshUser = vi.fn()
    mockedPost.mockResolvedValue({ data: { status: 'success' } })
    mockedUseAuth.mockReturnValue({
      user: makeUser({
        email: 'owner@example.com',
        display_name: 'Owner',
        training_data_consent: true,
        pdf_passwords: JSON.stringify(['1111', '2222']),
      }),
      refreshUser,
      requestMagicLink: vi.fn(),
      verifyToken: vi.fn(),
      logout: vi.fn(),
      loading: false,
      api: '/api',
    })

    render(<SettingsPage />)

    await userEvent.click(screen.getByRole('button', { name: /save settings/i }))

    await waitFor(() => {
      expect(mockedPost).toHaveBeenCalledWith(
        '/api/settings',
        expect.objectContaining({
          email: 'owner@example.com',
          display_name: 'Owner',
          training_data_consent: true,
          pdf_passwords: ['1111', '2222'],
        }),
      )
    })
    expect(refreshUser).toHaveBeenCalled()
  })

  it('requests account deletion for logged-in users', async () => {
    mockedPost.mockResolvedValue({ data: { status: 'success' } })
    mockedUseAuth.mockReturnValue({
      user: makeUser({ email: 'delete-me@example.com' }),
      refreshUser: vi.fn(),
      requestMagicLink: vi.fn(),
      verifyToken: vi.fn(),
      logout: vi.fn(),
      loading: false,
      api: '/api',
    })

    render(<SettingsPage />)

    await userEvent.click(screen.getByRole('button', { name: /request deletion/i }))

    expect(mockedPost).toHaveBeenCalledWith('/api/data-deletion/request', {
      email: 'delete-me@example.com',
    })
  })
})
