'use client'

import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'

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
  const [importPath, setImportPath] = useState('')
  const [selected, setSelected] = useState<string>('')
  const [validation, setValidation] = useState<ValidationResult | null>(null)

  const load = async () => {
    const data = await invoke<ConfigInfo[]>('load_all_configs')
    setList(data)
  }

  useEffect(() => {
    load()
  }, [])

  const handleImport = async () => {
    if (!importPath) return
    await invoke<string>('import_config', { sourcePath: importPath })
    setImportPath('')
    await load()
  }

  const handleValidate = async (p: string) => {
    const r = await invoke<ValidationResult>('validate_config', { configPath: p })
    setSelected(p)
    setValidation(r)
  }

  return (
    <div className="mx-auto max-w-3xl p-6 space-y-6">
      <h1 className="text-2xl font-semibold">配置管理</h1>

      <div className="rounded-lg border p-4 space-y-3">
        <div className="flex items-center gap-2">
          <input
            className="flex-1 rounded-md border px-3 py-2 text-sm"
            placeholder="输入要导入的 YAML 文件绝对路径"
            value={importPath}
            onChange={(e) => setImportPath(e.target.value)}
          />
          <button className="rounded-md border px-4 py-2 text-sm" onClick={handleImport}>
            导入配置
          </button>
        </div>
      </div>

      <div className="rounded-lg border p-4 space-y-3">
        <div className="text-sm text-muted-foreground">已检测到的配置文件</div>
        <div className="space-y-2">
          {list.map((c) => (
            <div key={c.path} className={`p-3 border rounded-md ${selected === c.path ? 'border-black dark:border-zinc-200' : ''}`}>
              <div className="flex items-center justify-between">
                <div>
                  <div className="text-sm font-medium">{c.name}</div>
                  <div className="text-xs text-zinc-500">{c.path}</div>
                </div>
                <div className="flex items-center gap-2">
                  <button className="rounded-md border px-3 py-1 text-sm" onClick={() => handleValidate(c.path)}>
                    验证
                  </button>
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
      </div>
    </div>
  )
}

