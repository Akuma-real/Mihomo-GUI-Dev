'use client'

import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
// 移除文本输入，改为仅通过系统对话框选择
 
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card'

type ConfigInfo = {
  name: string
  path: string
  size: number
  modified: string
}

type ValidationResult = {
  isValid: boolean
  warnings: string[]
  needsPrivilege: boolean
}

export default function ConfigPage() {
  const [list, setList] = useState<ConfigInfo[]>([])
  const [selectedPath, setSelectedPath] = useState('')
  const [selected, setSelected] = useState<string>('')
  const [validation, setValidation] = useState<ValidationResult | null>(null)

  const load = async () => {
    const data = await invoke<ConfigInfo[]>('load_all_configs')
    setList(data)
  }

  useEffect(() => {
    load()
  }, [])

  const pickFile = async () => {
    try {
      const { open } = await import('@tauri-apps/plugin-dialog')
      const file = await open({
        multiple: false,
        filters: [{ name: 'Mihomo 配置', extensions: ['yaml', 'yml'] }],
      })
      if (typeof file === 'string') {
        setSelectedPath(file)
        await invoke<string>('import_config', { sourcePath: file })
        await load()
      }
    } catch (e) {
      console.error(e)
      alert('选择或导入失败，请查看控制台日志')
    }
  }

  const handleValidate = async (p: string) => {
    const r = await invoke<ValidationResult>('validate_config', { configPath: p })
    setSelected(p)
    setValidation(r)
  }

  return (
    <div className="mx-auto max-w-3xl p-6 space-y-6">
      <h1 className="text-2xl font-semibold">配置管理</h1>

      <Card>
        <CardHeader>
          <CardTitle>导入配置</CardTitle>
          <CardDescription>通过系统对话框选择 YAML 文件</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="flex items-center gap-2">
            <Button onClick={pickFile} className="transition-transform active:scale-95 active:translate-y-px">
              选择 YAML 文件并导入
            </Button>
          </div>
          {selectedPath && (
            <div className="mt-2 text-xs text-zinc-500 truncate" title={selectedPath}>
              最近导入：{selectedPath}
            </div>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>已检测到的配置文件</CardTitle>
          <CardDescription>点击项目进行验证与查看提示</CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          <div className="space-y-2">
            {list.map((c) => (
              <div key={c.path} className={`p-3 border rounded-md ${selected === c.path ? 'border-black dark:border-zinc-200' : ''}`}>
                <div className="flex items-center justify-between">
                  <div>
                    <div className="text-sm font-medium">{c.name}</div>
                    <div className="text-xs text-zinc-500">{c.path}</div>
                  </div>
                  <div className="flex items-center gap-2">
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => handleValidate(c.path)}
                      className="transition-transform active:scale-95 active:translate-y-px"
                    >
                      验证
                    </Button>
                  </div>
                </div>
              </div>
            ))}
          </div>
          {validation && (
            <div className="mt-3 rounded-md bg-zinc-50 dark:bg-zinc-900 p-3 text-sm">
              <div>有效：{String(validation.isValid)}</div>
              <div>需权限：{String(validation.needsPrivilege)}</div>
              {validation.warnings.length > 0 && (
                <div className="mt-2 space-y-1">
                  {validation.warnings.map((w, i) => (
                    <div key={i} className="text-amber-600">• {w}</div>
                  ))}
                </div>
              )}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  )
}
