import { ref } from 'vue'
import type { ColData, FormatRule } from '@/types'
import { getData, submitExecute } from '@/api'

/** Excel 解析出来的列数据 */
export const columns = ref<ColData[]>([])
/** 是否已经从后端加载过 */
export const loaded = ref(false)
/** 选择的待改名文件夹路径：跨页面保留，回到命名格式页时回填到输入框 */
export const executePath = ref('')
/** 命名规则：跨页面保留，从分析页回到命名格式页时原样回填 */
export const formatRule = ref<FormatRule | null>(null)

/** 最近一次真正提交给后端的规则指纹，用于判断规则是否被改过 */
let appliedKey = ''

/** 加载列数据，force 为 true 时强制刷新 */
export async function loadColumns(force = false): Promise<ColData[]> {
    if (loaded.value && !force) return columns.value
    columns.value = await getData()
    loaded.value = true
    return columns.value
}

/********** 命名规则 **********/
/** 规则指纹：只取会影响改名结果的部分（段 id 只用于渲染，不参与比较） */
function ruleKey(rule: FormatRule, path: string): string {
    return JSON.stringify([
        path,
        rule.same,
        rule.sep,
        rule.customSep,
        rule.segments.map((s) => [s.type, s.value]),
    ])
}

/** 当前规则是否已经和后端里生效的一致（改过就需要重新提交才生效） */
export function ruleChanged(): boolean {
    const rule = formatRule.value
    if (!rule) return false
    return ruleKey(rule, executePath.value.trim()) !== appliedKey
}

/** 把当前规则提交给后端，使其立即生效 */
export async function applyFormatRule(
    path = executePath.value.trim(),
): Promise<{ ok: boolean; msg: string }> {
    const rule = formatRule.value
    const dir = path.trim()
    if (!dir) return { ok: false, msg: '请先选择包含待改名文件的文件夹' }
    if (!rule || rule.segments.length === 0) return { ok: false, msg: '命名构建区为空，请先构建命名规则' }

    try {
        const res = await submitExecute(dir, rule.segments.map((s) => s.value))
        if (res.code === 0) appliedKey = ruleKey(rule, dir)
        return { ok: res.code === 0, msg: res.msg }
    } catch (e) {
        return { ok: false, msg: `提交失败：${e}` }
    }
}

/** 清空命名规则（换了一份表格后，旧的规则已经失效） */
export function resetRule() {
    formatRule.value = null
    appliedKey = ''
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
    executePath.value = ''
    resetRule()
    resetStep(0)
}
