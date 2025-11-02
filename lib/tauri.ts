export function isTauri() {
  if (typeof window === 'undefined') return false
  // 检测常见的 Tauri 注入标记
  if ('__TAURI__' in window || '__TAURI_IPC__' in window || '__TAURI_METADATA__' in window) return true
  if (typeof navigator !== 'undefined' && /Tauri/i.test(navigator.userAgent || '')) return true
  return false
}
