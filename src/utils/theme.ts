import { ref, watch } from 'vue'

const STORAGE_KEY = 'formatname:dark'

const mql = typeof window.matchMedia === 'function'
    ? window.matchMedia('(prefers-color-scheme: dark)')
    : undefined

/** 读取本地偏好：'1' 暗色 / '0' 亮色 / null 未设置（跟随系统） */
function readStored(): boolean | null {
    try {
        const v = localStorage.getItem(STORAGE_KEY)
        return v === null ? null : v === '1'
    } catch {
        return null
    }
}

/** 是否处于暗色模式，初始值：本地偏好 > 系统偏好 */
export const isDark = ref(readStored() ?? mql?.matches ?? false)

/** 把状态同步到 <html class="dark">，同时驱动 Tailwind 的 dark: 与 Element Plus 暗色变量 */
function apply(dark: boolean) {
    document.documentElement.classList.toggle('dark', dark)
}

/** 切换亮/暗，并写入本地偏好 */
export function toggleDark() {
    isDark.value = !isDark.value
}

apply(isDark.value)

watch(isDark, (dark) => {
    apply(dark)
    try {
        localStorage.setItem(STORAGE_KEY, dark ? '1' : '0')
    } catch {
        /* 隐私模式下写入失败不影响使用 */
    }
})

// 用户未手动切换过时，跟随系统主题变化
mql?.addEventListener('change', (e) => {
    if (readStored() === null) isDark.value = e.matches
})
