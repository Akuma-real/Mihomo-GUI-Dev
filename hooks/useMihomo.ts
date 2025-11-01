import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'

type CoreStatus = 'running' | 'stopped' | 'error'

export function useMihomo() {
  const [status, setStatus] = useState<CoreStatus>('stopped')
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const start = async (configPath: string) => {
    setIsLoading(true)
    setError(null)
    try {
      await invoke('start_core', { configPath, needPrivilege: false })
      setStatus('running')
    } catch (err: any) {
      setError(err?.message ?? '启动失败')
      setStatus('error')
    } finally {
      setIsLoading(false)
    }
  }

  const stop = async () => {
    setIsLoading(true)
    setError(null)
    try {
      await invoke('stop_core')
      setStatus('stopped')
    } catch (err: any) {
      setError(err?.message ?? '停止失败')
    } finally {
      setIsLoading(false)
    }
  }

  useEffect(() => {
    const id = setInterval(async () => {
      try {
        const s = await invoke<string>('get_core_status')
        if (s === 'running' || s === 'stopped' || s === 'error') {
          setStatus(s)
        }
      } catch {}
    }, 1500)
    return () => clearInterval(id)
  }, [])

  return { status, isLoading, error, start, stop }
}

