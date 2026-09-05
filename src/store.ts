import { ref } from 'vue'
import type { ColData } from '@/types'
import { getData } from '@/api'

/** Excel 解析出来的列数据 */
export const columns = ref<ColData[]>([])
/** 是否已经从后端加载过 */
export const loaded = ref(false)

/** 加载列数据，force 为 true 时强制刷新 */
export async function loadColumns(force = false): Promise<ColData[]> {
  if (loaded.value && !force) return columns.value
  columns.value = await getData()
  loaded.value = true
  return columns.value
}

/********** 步骤进度 **********/
/** 已解锁的最大步骤序号（0 起）：只有到达过或更早的步骤可以直接跳转 */
export const maxStep = ref(0)

/** 记录已到达的步骤，取最大值，这样回退到前面的步骤也不会丢失进度 */
export function reachStep(step: number) {
  if (step > maxStep.value) maxStep.value = step
}

/** 重置步骤进度（重新提交 Excel 后需要重新走流程） */
export function resetStep(step = 0) {
  maxStep.value = step
}

/** 清空本地缓存的数据 */
export function resetColumns() {
  columns.value = []
  loaded.value = false
  resetStep(0)
}
