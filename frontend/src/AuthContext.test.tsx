import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import axios from 'axios'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { AuthProvider, useAuth } from './AuthContext'
import { makeUser } from './test/factories'

vi.mock('axios', () => ({
  default: {
    get: vi.fn(),
    post: vi.fn(),
  },
}))

const mockedGet = vi.mocked(axios.get)
const mockedPost = vi.mocked(axios.post)

const AuthProbe = () => {
  const { user, loading, requestMagicLink, verifyToken, refreshUser, logout } = useAuth()

  return (
    <div>
      <div data-testid="loading">{loading ? 'loading' : 'ready'}</div>
      <div data-testid="email">{user?.email ?? 'guest'}</div>
      <button onClick={() => requestMagicLink('new@example.com')}>request link</button>
      <button onClick={() => verifyToken('token-123')}>verify token</button>
      <button onClick={() => refreshUser()}>refresh user</button>
      <button onClick={() => logout()}>logout</button>
    </div>
  )
}

describe('AuthContext', () => {
  beforeEach(() => {
    localStorage.clear()
    mockedGet.mockReset()
    mockedPost.mockReset()
  })

  it('loads a saved user from local storage', async () => {
    localStorage.setItem('user_email', 'saved@example.com')
    mockedGet.mockResolvedValue({ data: makeUser({ email: 'saved@example.com' }) })

    render(
      <AuthProvider>
        <AuthProbe />
      </AuthProvider>,
    )

    expect(await screen.findByTestId('email')).toHaveTextContent('saved@example.com')
    expect(screen.getByTestId('loading')).toHaveTextContent('ready')
    expect(mockedGet).toHaveBeenCalledWith('/api/me?email=saved@example.com')
  })

  it('verifies tokens, persists the email, refreshes, and logs out', async () => {
    mockedPost.mockResolvedValueOnce({ data: makeUser({ email: 'verified@example.com' }) })
    mockedGet.mockResolvedValueOnce({ data: makeUser({ email: 'refreshed@example.com' }) })

    render(
      <AuthProvider>
        <AuthProbe />
      </AuthProvider>,
    )

    await waitFor(() => expect(screen.getByTestId('loading')).toHaveTextContent('ready'))

    await userEvent.click(screen.getByRole('button', { name: /verify token/i }))
    expect(await screen.findByTestId('email')).toHaveTextContent('verified@example.com')
    expect(localStorage.getItem('user_email')).toBe('verified@example.com')

    await userEvent.click(screen.getByRole('button', { name: /refresh user/i }))
    expect(await screen.findByTestId('email')).toHaveTextContent('refreshed@example.com')

    await userEvent.click(screen.getByRole('button', { name: /logout/i }))
    expect(screen.getByTestId('email')).toHaveTextContent('guest')
    expect(localStorage.getItem('user_email')).toBeNull()
  })

  it('requests a magic link through the backend API', async () => {
    mockedPost.mockResolvedValueOnce({ data: { status: 'success' } })

    render(
      <AuthProvider>
        <AuthProbe />
      </AuthProvider>,
    )

    await waitFor(() => expect(screen.getByTestId('loading')).toHaveTextContent('ready'))
    await userEvent.click(screen.getByRole('button', { name: /request link/i }))

    expect(mockedPost).toHaveBeenCalledWith('/api/auth/magic-link', {
      email: 'new@example.com',
    })
  })
})
