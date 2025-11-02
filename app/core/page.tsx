'use client'

import { useState } from 'react'
import { useMihomo } from '@/hooks/useMihomo'
import VersionSwitcher from './components/VersionSwitcher'
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
//
import { invoke } from '@tauri-apps/api/core'

export default function CorePage() {
  const { status, isLoading, error, start, stop } = useMihomo()
  const [configPath, setConfigPath] = useState('')
  type TunHint = { enabled: boolean; has_permission: boolean; platform: string; suggested_cmd?: string; message: string }
  const [tunHint, setTunHint] = useState<null | TunHint>(null)
  const [svcStatus, setSvcStatus] = useState<string>('unknown')

  const pickConfig = async () => {
    try {
      const { open } = await import('@tauri-apps/plugin-dialog')
      const file = await open({ multiple: false, filters: [{ name: 'Mihomo 配置', extensions: ['yaml', 'yml'] }] })
      if (typeof file === 'string') {
        setConfigPath(file)
        // 选择后检查 TUN 提示
        try {
          const hint = await invoke<TunHint>('check_tun_hint', { configPath: file })
          setTunHint(hint)
        } catch (e) {
          console.warn('check_tun_hint failed', e)
        }
      }
    } catch (e) {
      console.error(e)
      alert('选择配置失败，请查看控制台日志')
    }
  }

  return (
    <div className="mx-auto max-w-3xl p-6 space-y-6">
      <h1 className="text-2xl font-semibold">Mihomo 内核</h1>

      <VersionSwitcher />

      <Card>
        <CardHeader>
          <CardTitle>启动控制</CardTitle>
          <CardDescription>选择配置路径，启动或停止内核</CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
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
            <Button onClick={pickConfig} className="transition-transform active:scale-95 active:translate-y-px">
              选择配置文件
            </Button>
            <span className="text-xs text-zinc-500 truncate" title={configPath}>
              {configPath ? `已选择：${configPath}` : '未选择'}
            </span>
          </div>

          {tunHint && tunHint.enabled && !tunHint.has_permission && (
            <div className="rounded-md border border-amber-300 bg-amber-50 text-amber-900 p-3 text-sm">
              <div className="font-medium mb-1">检测到启用 TUN，需要管理员权限</div>
              <div className="mb-1">{tunHint.message}</div>
              <div className="text-xs text-amber-800">提示：为避免频繁弹出系统密码框，我们不再提供命令提权。可在配置中关闭 TUN，或以管理员权限运行应用。</div>
            </div>
          )}

          <div className="flex items-center gap-2">
            <Button
              disabled={!configPath || isLoading}
              onClick={() => start(configPath)}
              className="transition-transform active:scale-95 active:translate-y-px"
            >
              启动
            </Button>
            <Button
              variant="outline"
              disabled={isLoading}
              onClick={stop}
              className="transition-transform active:scale-95 active:translate-y-px"
            >
              停止
            </Button>
          </div>

          {error && <p className="text-sm text-red-600">{error}</p>}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>系统服务（Linux / systemd）</CardTitle>
          <CardDescription>安装为 root 服务，GUI 仅连接 external-controller</CardDescription>
        </CardHeader>
        <CardContent className="space-y-2">
          <div className="text-sm text-muted-foreground">
            当前状态：{svcStatus || 'unknown'}
          </div>
          <div className="flex items-center gap-2">
            <Button
              variant="outline"
              onClick={async () => {
                try {
                  const s = await invoke<string>('systemd_service_status')
                  setSvcStatus(s)
                } catch (e) {
                  setSvcStatus('unknown')
                }
              }}
              className="transition-transform active:scale-95 active:translate-y-px"
            >
              刷新状态
            </Button>
            <Button
              onClick={async () => {
                try {
                  if (!configPath) {
                    alert('请先选择配置文件')
                    return
                  }
                  await invoke('install_systemd_service', { configPath })
                  const s = await invoke<string>('systemd_service_status')
                  setSvcStatus(s)
                  alert('已安装并启动服务（/usr/local/bin/mihomo + /etc/mihomo/config.yaml）')
                } catch (e) {
                  console.error(e)
                  alert('安装失败，请查看终端日志')
                }
              }}
              className="transition-transform active:scale-95 active:translate-y-px"
            >
              安装并启动服务
            </Button>
          </div>
          <div className="flex items-center gap-4 text-sm">
            <label className="inline-flex items-center gap-2">
              <input type="checkbox" checked={removeBin} onChange={(e) => setRemoveBin(e.target.checked)} />
              卸载时删除 /usr/local/bin/mihomo
            </label>
            <label className="inline-flex items-center gap-2">
              <input type="checkbox" checked={removeCfg} onChange={(e) => setRemoveCfg(e.target.checked)} />
              卸载时删除 /etc/mihomo/config.yaml
            </label>
          </div>
          <div className="flex items-center gap-2">
            <Button
              variant="destructive"
              onClick={async () => {
                try {
                  await invoke('uninstall_systemd_service', { deleteBinary: removeBin, deleteConfig: removeCfg })
                  const s = await invoke<string>('systemd_service_status')
                  setSvcStatus(s)
                  alert('已卸载服务')
                } catch (e) {
                  console.error(e)
                  alert('卸载失败，请查看终端日志')
                }
              }}
              className="transition-transform active:scale-95 active:translate-y-px"
            >
              卸载服务
            </Button>
          </div>
          <div className="text-xs text-zinc-500">
            说明：执行时会弹一次管理员授权（pkexec）。安装会复制二进制到 /usr/local/bin/mihomo，并将配置复制到 /etc/mihomo/config.yaml；卸载可选择同时删除二进制与配置。
          </div>
        </CardContent>
      </Card>
    </div>
  )
}
