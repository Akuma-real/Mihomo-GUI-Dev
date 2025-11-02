'use client'

import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { isTauri } from '@/lib/tauri'
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Progress } from '@/components/ui/progress'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'

export default function VersionSwitcher() {
  const [channel, setChannel] = useState<'stable' | 'dev'>('stable')
  const [currentCorePath, setCurrentCorePath] = useState<string>('')
  const [latest, setLatest] = useState<string>('')
  const [loading, setLoading] = useState(false)
  const [installing, setInstalling] = useState(false)
  const [progress, setProgress] = useState(0)
  const [progressStage, setProgressStage] = useState('')
  const [progressError, setProgressError] = useState<string | null>(null)
  const [installDir, setInstallDir] = useState('')

  useEffect(() => {
    // 直接读取默认路径用于显示（后端启动时已尝试自动采用该路径）
    invoke<string | null>('get_default_core_path').then((d) => { if (d) setCurrentCorePath(d) }).catch(() => {})
    invoke<string>('get_core_install_dir').then(setInstallDir).catch(() => {})
    ;(async () => {
      const unlisten = await listen<{ stage: string; progress: number; message?: string }>(
        'version_install_progress',
        (e) => {
          setProgress(e.payload.progress)
          setProgressStage(e.payload.stage)
          setProgressError(e.payload.stage === '错误' ? e.payload.message ?? '未知错误' : null)
        }
      )
      return () => {
        unlisten()
      }
    })()
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

  const downloadAndInstall = async () => {
    setInstalling(true)
    setProgress(0)
    setProgressStage('')
    setProgressError(null)
    try {
      const installedPath = await invoke<string>('download_install_latest', { channel })
      setCurrentCorePath(installedPath)
    } catch (e) {
      console.error(e)
      alert('下载/安装失败，请确认已在 Tauri 桌面端运行，并查看控制台/终端日志')
    } finally {
      setInstalling(false)
    }
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>内核版本管理</CardTitle>
        <CardDescription>选择渠道、查询并安装 Mihomo 内核</CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="flex items-center gap-3">
          <div className="min-w-28 text-sm text-muted-foreground">发布渠道</div>
          <div className="w-40">
            <Select value={channel} onValueChange={(v) => setChannel(v as 'stable' | 'dev')}>
              <SelectTrigger className="transition-transform active:scale-95 active:translate-y-px">
                <SelectValue placeholder="选择渠道" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="stable">稳定版</SelectItem>
                <SelectItem value="dev">开发版</SelectItem>
              </SelectContent>
            </Select>
          </div>
          <Button
            variant="outline"
            size="sm"
            onClick={loadLatest}
            disabled={loading}
            className="transition-transform active:scale-95 active:translate-y-px"
          >
            查询最新版本
          </Button>
          {latest && <span className="text-sm">最新版本: {latest}</span>}
        </div>

        <div className="space-y-2">
          <div className="text-sm text-muted-foreground">当前内核路径：{currentCorePath || '未设置'}</div>
          {installDir && (
            <div className="text-xs text-zinc-500">安装目录：{installDir}</div>
          )}
          <div className="flex items-center gap-2">
            <Button
              onClick={downloadAndInstall}
              disabled={installing}
              className="transition-transform active:scale-95 active:translate-y-px"
            >
              {installing ? '安装中…' : '下载并安装'}
            </Button>
            {!currentCorePath && (
              <Button
                variant="outline"
                onClick={async () => {
                  const d = await invoke<string | null>('get_default_core_path').catch(() => null)
                  if (d) setCurrentCorePath(d)
                }}
                className="transition-transform active:scale-95 active:translate-y-px"
              >
                刷新默认路径
              </Button>
            )}
          </div>
          {(installing || !isTauri()) && (
            <div className="space-y-1">
              <Progress value={progress} />
              <div className="text-xs text-zinc-600 dark:text-zinc-400">
                {isTauri() ? `${progressStage || '准备中…'} ${progress}%` : '提示：请以 Tauri 桌面模式运行以启用下载与安装'}
              </div>
              {progressError && <div className="text-xs text-red-600">错误：{progressError}</div>}
            </div>
          )}
        </div>
      </CardContent>
    </Card>
  )
}
