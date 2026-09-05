import {isTauri} from '@tauri-apps/api/core'
import {writeText as tauriWriteText} from '@tauri-apps/plugin-clipboard-manager'

/** 浏览器环境的兜底复制 */
async function webWriteText(text: string): Promise<void> {
    if (navigator.clipboard?.writeText) {
        try {
            await navigator.clipboard.writeText(text)
            return
        } catch {
            // 权限被拒或非安全上下文时继续走下面的兜底方案
        }
    }

    const textarea = document.createElement('textarea')
    textarea.value = text
    textarea.style.position = 'fixed'
    textarea.style.top = '-9999px'
    textarea.style.opacity = '0'
    document.body.appendChild(textarea)
    textarea.select()
    try {
        if (!document.execCommand('copy')) throw new Error('浏览器拒绝了复制操作')
    } finally {
        textarea.remove()
    }
}

/** 复制文本：优先使用 Tauri 剪贴板插件，失败时回退到浏览器剪贴板 */
export async function copyText(text: string): Promise<void> {
    if (isTauri()) {
        try {
            await tauriWriteText(text)
            return
        } catch (e) {
            console.warn('[clipboard] Tauri 复制失败，回退到浏览器剪贴板：', e)
        }
    }
    await webWriteText(text)
}
