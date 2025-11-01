'use client'

import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'

export default function VersionSwitcher() {
  const [channel, setChannel] = useState<'stable' | 'dev'>('stable')
  const [currentCorePath, setCurrentCorePath] = useState<string>('')
  const [inputPath, setInputPath] = useState('')
  const [latest, setLatest] = useState<string>('')
  const [loading, setLoading] = useState(false)

  useEffect(() => {
    invoke<string | null>('get_core_path').then((p) => setCurrentCorePath(p ?? ''))
  }, [])

  const loadLatest = async () => {
    setLoading(true)
    try {
      const v = await invoke<string>('fetch_latest_version', { channel })
      setLatest(v)
    } finally {
      setLoading(false)
    }
  }

  const setCore = async () => {
    if (!inputPath) return
    await invoke('set_core_path', { corePath: inputPath })
    setCurrentCorePath(inputPath)
  }

  return (
    <div className="rounded-lg border p-4 space-y-3">
      <div className="flex items-center gap-3">
        <span className="text-sm text-muted-foreground">发布渠道</span>
        <select
          className="border rounded-md px-2 py-1 text-sm"
          value={channel}
          onChange={(e) => setChannel(e.target.value as any)}
        >
          <option value="stable">稳定版</option>
          <option value="dev">开发版</option>
        </select>
        <button className="border rounded-md px-3 py-1 text-sm" onClick={loadLatest} disabled={loading}>
          查询最新版本
        </button>
        {latest && <span className="text-sm">最新版本: {latest}</span>}
      </div>

      <div className="space-y-2">
        <div className="text-sm text-muted-foreground">当前内核路径：{currentCorePath || '未设置'}</div>
        <div className="flex items-center gap-2">
          <input
            className="flex-1 rounded-md border px-3 py-2 text-sm"
            placeholder="输入已下载的 mihomo 可执行文件路径"
            value={inputPath}
            onChange={(e) => setInputPath(e.target.value)}
          />
          <button className="rounded-md border px-4 py-2 text-sm" onClick={setCore}>
            设置内核路径
          </button>
        </div>
      </div>
    </div>
  )
}

