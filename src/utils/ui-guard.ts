/** 允许保留原生交互的元素：输入框等可编辑区域 */
const EDITABLE_SELECTOR =
    'input, textarea, select, [contenteditable=""], [contenteditable="true"], .selectable'

/** 事件目标是否位于可编辑区域内 */
function isEditable(target: EventTarget | null): boolean {
    return target instanceof Element && !!target.closest(EDITABLE_SELECTOR)
}

/**
 * 全局 UI 防护：禁用右键菜单、F7 插入光标浏览。
 * 说明：Web 端只能拦截「交给渲染进程优先处理」的快捷键，
 * F7（插入光标浏览）属于这一类，因此 preventDefault 生效。
 */
export function installUiGuard(): void {
    // 禁用右键菜单（可编辑区域保留，方便粘贴）
    window.addEventListener('contextmenu', (e) => {
        if (isEditable(e.target)) return
        e.preventDefault()
    })

    // 禁用 F7 插入光标浏览：必须在事件到达浏览器默认处理前拦截
    window.addEventListener(
        'keydown',
        (e) => {
            if (e.key === 'F7' || e.keyCode === 118) {
                e.preventDefault()
                e.stopPropagation()
            }
        },
        { capture: true },
    )
}
