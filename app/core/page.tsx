'use client'

import { useState } from 'react'
import { useMihomo } from '@/hooks/useMihomo'
import VersionSwitcher from './components/VersionSwitcher'

export default function CorePage() {
  const { status, isLoading, error, start, stop } = useMihomo()
  const [configPath, setConfigPath] = useState('')

  return (
    <div className="mx-auto max-w-2xl p-6 space-y-6">
      <h1 className="text-2xl font-semibold">Mihomo 内核</h1>

      <VersionSwitcher />

      <div className="rounded-lg border p-4 space-y-3">
        <div className="text-sm text-muted-foreground">
          当前状态：
          <span
            className={
              status === 'running'
                ? 'text-green-600'
                : status === 'error'
                ? 'text-red-600'
                : 'text-zinc-600'
            }
          >
            {' '}
            {status}
          </span>
        </div>

        <div className="flex items-center gap-2">
          <input
            className="flex-1 rounded-md border px-3 py-2 text-sm"
            placeholder="配置文件路径，例如 /home/user/mihomo.yaml"
            value={configPath}
            onChange={(e) => setConfigPath(e.target.value)}
          />
          <button
            className="rounded-md bg-black px-4 py-2 text-sm text-white disabled:opacity-50 dark:bg-zinc-100 dark:text-black"
            disabled={!configPath || isLoading}
            onClick={() => start(configPath)}
          >
            启动
          </button>
          <button
            className="rounded-md border px-4 py-2 text-sm disabled:opacity-50"
            disabled={isLoading}
            onClick={stop}
          >
            停止
          </button>
        </div>

        {error && <p className="text-sm text-red-600">{error}</p>}
      </div>
    </div>
  )
}
