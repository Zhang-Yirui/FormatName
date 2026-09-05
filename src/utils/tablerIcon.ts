import { h, type Component } from 'vue'
import { ElIcon } from 'element-plus'

interface IconOptions {
    size?: number | string
    color?: string
    stroke?: number | string
}

/**
 * 将任意图标组件包装成 el-button 的 icon 可用形式
 */
export function tablerIcon(
    icon: Component,
    options: IconOptions = {}
) {
    return h(ElIcon, options, {
        default: () => h(icon)
    })
}