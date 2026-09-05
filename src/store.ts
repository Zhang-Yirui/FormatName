import { ref } from 'vue'
import type { ColData } from '@/types'
import { getData } from '@/api'

/** Excel 解析出来的列数据 */
export const columns = ref<ColData[]>([])

/** 是否已经从后端加载过 */
export const loaded = ref(false)

/** 加载列数据, force 为 true 时强制刷新 */
export async function loadColumns(force = false): Promise<ColData[]> {
  if (loaded.value && !force) return columns.value
  columns.value = await getData()
  loaded.value = true
  return columns.value
}

/** 清空本地缓存的数据 */
export function resetColumns() {
  columns.value = []
  loaded.value = false
}
