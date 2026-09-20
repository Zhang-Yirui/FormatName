<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { open } from '@tauri-apps/plugin-dialog'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { ElMessage } from 'element-plus'
import { ArrowLeft, FolderOpened, RefreshRight } from '@element-plus/icons-vue'
import { isDir, submitExecute } from '@/api'
import { columns } from '@/store'
import type { ExecuteItem } from '@/types'
import { STEPS } from '@/constant'

const router = useRouter()

/** 默认连字符 */
const DEFAULT_SEP = '-'
/** 统一分隔符中的“自定义”选项标识 */
const CUSTOM = '__custom__'

/** 命名段类型：字段 / 分隔符 / 自定义文本 */
type SegType = 'keyword' | 'connector' | 'custom'

interface Segment {
  id: string
  type: SegType
  /** keyword 为列序号，connector / custom 为文本 */
  value: number | string
}

interface SepOption {
  label: string
  value: string
  tip: string
}

const SEPARATORS: SepOption[] = [
  { label: '无', value: '', tip: '不使用分隔符，各段直接拼接' },
  { label: '-', value: '-', tip: '连字符' },
  { label: '_', value: '_', tip: '下划线' },
  { label: '+', value: '+', tip: '加号' },
  { label: '␣', value: ' ', tip: '空格' },
  { label: '.', value: '.', tip: '点号' },
]

/** 统一分隔符可选值（多一个“自定义”） */
const SEP_OPTIONS: SepOption[] = [
  ...SEPARATORS,
  { label: '自定义', value: CUSTOM, tip: '自定义分隔符，可输入任意字符' },
]

/** 分隔符的展示文本 */
function displaySep(value: string): string {
  if (value === '') return '无'
  if (value === ' ') return '␣'
  return value
}

let seed = 0
function newId(): string {
  return `seg_${++seed}`
}

function toSegment(item: ExecuteItem): Segment {
  if (typeof item === 'number') return { id: newId(), type: 'keyword', value: item }
  return { id: newId(), type: 'connector', value: item ?? '' }
}

/** 初始命名格式：把关键字页选中的关键字全部加入，重复次数越少优先级越高 */
function initSegments(): Segment[] {
  const keywords = columns.value
      .map((col, index) => ({ col, index }))
      .filter(({ col }) => col.isKeyWord && col.display)
      .sort((a, b) => a.col.delta - b.col.delta)
      .map(({ index }) => index)

  const items: ExecuteItem[] = []
  if (keywords.length === 0) {
    items.push(...(columns.value.length > 1 ? [0, DEFAULT_SEP, 1] : [0]))
  } else {
    keywords.forEach((index, i) => {
      if (i > 0) items.push(DEFAULT_SEP)
      items.push(index)
    })
  }
  return items.map(toSegment)
}

/** 构建区中的命名段 */
const segments = ref<Segment[]>(initSegments())
/** 是否统一分隔符 */
const same = ref(true)
/** 统一的分隔符（选中的选项值） */
const sep = ref(DEFAULT_SEP)
/** 自定义分隔符内容 */
const customSep = ref('')
/** 自定义连接符输入 */
const customConnectorInput = ref('')
/** 自定义文本输入 */
const customTextInput = ref('')
/** 已添加过的自定义连接符 */
const customConnectors = ref<string[]>([])
/** 要改名的文件夹 */
const path = ref('')
const loading = ref(false)

/** 当前生效的分隔符 */
const activeSep = computed(() => (sep.value === CUSTOM ? customSep.value : sep.value))

/** 提交给后端的命名格式 */
const picked = computed<ExecuteItem[]>(() => segments.value.map((s) => s.value))

function applySep() {
  segments.value.forEach((s) => {
    if (s.type === 'connector') s.value = activeSep.value
  })
}

watch([sep, customSep], () => {
  if (same.value) applySep()
})

watch(same, (val) => {
  if (val) applySep()
})

/* ==================== 添加 / 移除 ==================== */
function addKeyword(index: number) {
  const col = columns.value[index]
  if (!col) return
  if (!col.display) {
    ElMessage.warning(`「${col.key}」不能用于命名：${col.reason}`)
    return
  }
  segments.value.push({ id: newId(), type: 'keyword', value: index })
}

function addConnector(value: string) {
  segments.value.push({ id: newId(), type: 'connector', value: same.value ? activeSep.value : value })
}

function addCustomConnector() {
  const val = customConnectorInput.value.trim()
  if (!val) return
  if (!customConnectors.value.includes(val)) customConnectors.value.push(val)
  addConnector(val)
  customConnectorInput.value = ''
}

function addCustomText() {
  const val = customTextInput.value.trim()
  if (!val) return
  segments.value.push({ id: newId(), type: 'custom', value: val })
  customTextInput.value = ''
}

function removeSegment(index: number) {
  segments.value.splice(index, 1)
}

function clearSegments() {
  segments.value = []
}

/* ==================== 拖拽 ==================== */
/* Tauri 窗口默认开启原生拖拽处理（ExcelView 靠它拿到文件完整路径），
   它会吞掉 HTML5 的 dragstart / dragover / drop 事件，
   所以这里统一用指针事件自己实现拖拽，浏览器与 Tauri 窗口表现一致。 */

interface DragState {
  /** 从左侧面板拖入时的载荷；构建区内部排序时为空 */
  payload: string
  /** 构建区内部排序时的原序号，-1 表示新增 */
  from: number
  label: string
}

/** 当前拖拽状态 */
const drag = ref<DragState | null>(null)
/** 拖拽是否已越过阈值（区分点击与拖拽） */
const dragging = ref(false)
/** 跟随光标的浮层 */
const ghost = ref({ x: 0, y: 0, label: '' })
/** 落点序号（0..length），null 表示不在构建区内 */
const dropIndex = ref<number | null>(null)
/** 构建区容器 */
const dropArea = ref<HTMLElement | null>(null)

let startPoint = { x: 0, y: 0 }
let moved = false

function startDrag(state: DragState, e: PointerEvent) {
  if (e.button !== 0) return
  // 不用 preventDefault，否则会连带屏蔽 click / dblclick（双击添加）；
  // 拖拽期间改为暂时禁止页面选中文本
  document.body.style.userSelect = 'none'
  drag.value = state
  ghost.value = { x: e.clientX, y: e.clientY, label: state.label }
  startPoint = { x: e.clientX, y: e.clientY }
  moved = false
  dragging.value = false
  dropIndex.value = null
  window.addEventListener('pointermove', onDragMove)
  window.addEventListener('pointerup', endDrag)
  window.addEventListener('pointercancel', endDrag)
}

/** 根据光标位置计算插入序号：命中某段则按左右半边决定插到它前面还是后面 */
function computeDropIndex(x: number, y: number): number | null {
  const area = dropArea.value
  if (!area) return null
  const rect = area.getBoundingClientRect()
  if (x < rect.left || x > rect.right || y < rect.top || y > rect.bottom) return null

  const nodes = Array.from(area.querySelectorAll<HTMLElement>('[data-seg]'))
  for (let i = 0; i < nodes.length; i++) {
    const r = nodes[i]!.getBoundingClientRect()
    if (y >= r.top && y <= r.bottom && x >= r.left && x <= r.right) {
      return x < r.left + r.width / 2 ? i : i + 1
    }
  }
  // 落在间隙或指示条上时保持上一次的落点，否则指示条插入后会来回抖动
  return dropIndex.value ?? nodes.length
}

function onDragMove(e: PointerEvent) {
  if (!drag.value) return
  ghost.value = { x: e.clientX, y: e.clientY, label: drag.value.label }
  if (!moved && Math.hypot(e.clientX - startPoint.x, e.clientY - startPoint.y) < 4) return
  moved = true
  dragging.value = true
  dropIndex.value = computeDropIndex(e.clientX, e.clientY)
}

function endDrag() {
  window.removeEventListener('pointermove', onDragMove)
  window.removeEventListener('pointerup', endDrag)
  window.removeEventListener('pointercancel', endDrag)
  document.body.style.userSelect = ''

  const state = drag.value
  const target = dropIndex.value
  drag.value = null
  dragging.value = false
  moved = false
  dropIndex.value = null
  if (!state || target === null) return
  commitDrag(state, target)
}

onBeforeUnmount(endDrag)

function commitDrag(state: DragState, target: number) {
  // 构建区内部排序
  if (state.from >= 0) {
    if (target === state.from || target === state.from + 1) return
    const [item] = segments.value.splice(state.from, 1)
    if (item) segments.value.splice(target > state.from ? target - 1 : target, 0, item)
    return
  }
  // 从左侧面板拖入
  const seg = createFromPayload(state.payload)
  if (seg) segments.value.splice(Math.min(target, segments.value.length), 0, seg)
}

/** 把左侧面板拖来的数据转成命名段 */
function createFromPayload(raw: string): Segment | null {
  if (raw.startsWith('kw_')) {
    const index = Number(raw.slice(3))
    const col = columns.value[index]
    if (!col) return null
    if (!col.display) {
      ElMessage.warning(`「${col.key}」不能用于命名：${col.reason}`)
      return null
    }
    return { id: newId(), type: 'keyword', value: index }
  }
  if (raw.startsWith('conn_')) {
    return { id: newId(), type: 'connector', value: same.value ? activeSep.value : raw.slice(5) }
  }
  if (raw.startsWith('txt_')) {
    return { id: newId(), type: 'custom', value: raw.slice(4) }
  }
  return null
}

/* ==================== 展示与预览 ==================== */
function segLabel(seg: Segment): string {
  if (seg.type === 'keyword') {
    return typeof seg.value === 'number' ? (columns.value[seg.value]?.key ?? String(seg.value)) : String(seg.value)
  }
  if (seg.type === 'custom') return `"${seg.value}"`
  return displaySep(String(seg.value ?? ''))
}

/** 格式串里的一段（空分隔符不显示文字） */
function segFormat(seg: Segment): string {
  if (seg.type === 'connector' && seg.value === '') return ''
  return segLabel(seg)
}

function segExample(seg: Segment): string {
  if (seg.type === 'keyword' && typeof seg.value === 'number') {
    return columns.value[seg.value]?.values[0] ?? ''
  }
  return String(seg.value ?? '')
}

function preview(suffix: (seg: Segment) => string): string {
  const text = segments.value.map(suffix).join('')
  return text ? text + '.xxx' : '（空）'
}

const formatText = computed(() => preview(segFormat))

const exampleText = computed(() => preview(segExample))

async function pickDir() {
  const selected = await open({
    directory: true,
    multiple: false,
    title: '选择包含待改名文件的文件夹',
  })
  if (typeof selected === 'string' && selected) {
    path.value = selected
  }
}

/* ==================== 拖拽选择文件夹 ==================== */
/* 与第一步导入 Excel 一样走 Tauri 原生拖拽事件：只有原生事件能拿到完整路径，
   浏览器给的 File 只有文件名。原生事件是窗口级的，所以要按落点判断是不是拖在
   “浏览”按钮所在的虚线框内（Windows 下 position 是 ScreenToClient 后的物理像素）。 */

/** 拖放区（含浏览按钮）是否高亮 */
const dirDragging = ref(false)
/** 拖放区容器 */
const dirZone = ref<HTMLElement | null>(null)
let unlistenDragDrop: (() => void) | undefined
/** 本次拖拽拿到的坐标是否可信（不可信时退化为“拖到哪都接收”） */
let dragCoordsUsable = false

/** 落点是否落在拖放区内：物理像素与 CSS 像素两种解释都试一次 */
function hitDirZone(x: number, y: number): boolean {
  const zone = dirZone.value
  if (!zone) return false
  const rect = zone.getBoundingClientRect()
  const ratio = window.devicePixelRatio || 1
  const hit = (px: number, py: number) =>
      px >= rect.left && px <= rect.right && py >= rect.top && py <= rect.bottom
  return hit(x, y) || hit(x / ratio, y / ratio)
}

/** 客户端坐标应当落在视口范围内，否则说明坐标系不一致（如拿到的是屏幕坐标） */
function coordsUsable(x: number, y: number): boolean {
  const ratio = window.devicePixelRatio || 1
  const inView = (px: number, py: number) =>
      px >= 0 && py >= 0 && px <= window.innerWidth && py <= window.innerHeight
  return inView(x, y) || inView(x / ratio, y / ratio)
}

/** 浏览器调试模式下没有原生事件，也就拿不到文件夹完整路径，只能提示 */
function onBrowserDrop() {
  dirDragging.value = false
  ElMessage.info('浏览器调试模式无法获取文件夹完整路径，请点击“浏览”选择')
}

async function setDroppedDir(p: string, inside: boolean) {
  let isDirectory: boolean | null = null
  try {
    isDirectory = await isDir(p)
  } catch {
    /* is_dir 命令不可用时不做拦截，交给提交时由后端校验 */
  }

  if (isDirectory === false) {
    ElMessage.warning('请拖入文件夹，而不是文件')
    return
  }
  // 坐标不可信时（平台/版本差异）不再挑剔落点，避免“拖了没反应”
  if (inside || !dragCoordsUsable) {
    path.value = p
    return
  }
  ElMessage.info('请把文件夹拖到“浏览”按钮上')
}

onMounted(async () => {
  try {
    unlistenDragDrop = await getCurrentWebview().onDragDropEvent(({ payload }) => {
      // position 缺失时按 (-1,-1) 处理，此时坐标不可信，会退化为“拖到哪都接收”
      const x = payload.type === 'leave' ? -1 : (payload.position?.x ?? -1)
      const y = payload.type === 'leave' ? -1 : (payload.position?.y ?? -1)

      if (payload.type === 'enter') {
        dragCoordsUsable = coordsUsable(x, y)
        dirDragging.value = hitDirZone(x, y)
        return
      }
      if (payload.type === 'over') {
        dirDragging.value = hitDirZone(x, y)
        return
      }
      if (payload.type === 'drop') {
        const inside = hitDirZone(x, y)
        dirDragging.value = false
        const p = payload.paths[0]
        if (p) void setDroppedDir(p, inside)
        return
      }
      dirDragging.value = false
    })
  } catch {
    /* 非 Tauri 环境（浏览器调试）忽略原生拖拽 */
  }
})

onBeforeUnmount(() => unlistenDragDrop?.())

async function submit() {
  if (!path.value.trim()) {
    ElMessage.warning('请先选择包含待改名文件的文件夹')
    return
  }
  loading.value = true
  try {
    const res = await submitExecute(path.value.trim(), picked.value)
    if (res.code === 0) {
      ElMessage.success(res.msg)
      await router.push('/analysis')
    } else {
      ElMessage.error(res.msg)
    }
  } catch (e) {
    ElMessage.error(`提交失败：${e}`)
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <div class="page-view">
    <div class="page-header">
      <div>
        <h1 class="page-title">{{ STEPS[2].title }}</h1>
        <p class="page-desc">{{ STEPS[2].hint }}</p>
      </div>
      <el-button text type="primary" :icon="RefreshRight" @click="router.push('/excel')">
        重新选择 Excel
      </el-button>
    </div>

    <el-card shadow="never" class="!rounded-2xl dark:bg-slate-800 dark:border-slate-700">
      <div class="grid grid-cols-1 items-start gap-5 lg:grid-cols-[290px_minmax(0,1fr)]">
        <!-- 左侧：来源面板 -->
        <div class="space-y-4">
          <!-- 表格字段 -->
          <div class="rounded-xl border border-slate-200 bg-white p-3.5 dark:border-slate-700 dark:bg-slate-800/60">
            <h4 class="mb-2.5 text-[11px] font-semibold uppercase tracking-wide text-slate-400 dark:text-slate-500">
              表格字段
            </h4>
            <el-scrollbar max-height="248">
              <div class="flex flex-col gap-1.5 pr-1">
                <div
                    v-for="(col, index) in columns"
                    :key="index"
                    class="flex select-none items-center gap-2 rounded-lg border border-slate-200 bg-slate-50 px-2.5 py-2 transition hover:border-brand/50 hover:bg-blue-50 dark:border-slate-700 dark:bg-slate-700/40 dark:hover:border-brand/60 dark:hover:bg-blue-500/15"
                    :class="
                    col.display
                      ? 'cursor-grab touch-none active:cursor-grabbing'
                      : 'cursor-not-allowed opacity-50'
                  "
                    @pointerdown="col.display && startDrag({ payload: 'kw_' + index, from: -1, label: col.key }, $event)"
                    @dblclick="addKeyword(index)"
                >
                  <span class="shrink-0 text-xs text-brand dark:text-brand-light">▣</span>
                  <span class="min-w-0 flex-1 truncate text-[13px] font-medium text-slate-700 dark:text-slate-300">
                    {{ col.key }}
                  </span>
                  <el-tooltip v-if="!col.display" :content="col.reason" placement="top">
                    <el-tag size="small" type="danger" effect="dark">非法</el-tag>
                  </el-tooltip>
                  <span v-else class="shrink-0 text-[10.5px] text-slate-300 dark:text-slate-600">双击添加</span>
                </div>
              </div>
            </el-scrollbar>
          </div>

          <!-- 自定义文本 -->
          <div class="rounded-xl border border-slate-200 bg-white p-3.5 dark:border-slate-700 dark:bg-slate-800/60">
            <h4 class="mb-2.5 text-[11px] font-semibold uppercase tracking-wide text-slate-400 dark:text-slate-500">
              自定义文本
            </h4>
            <div class="flex gap-2">
              <el-input
                  v-model="customTextInput"
                  size="small"
                  placeholder="输入固定文本..."
                  @keyup.enter="addCustomText"
              />
              <el-button size="small" type="primary" plain @click="addCustomText">添加</el-button>
            </div>
            <p class="mt-2 text-[11px] text-slate-400 dark:text-slate-500">
              固定文本会原样出现在每个新名字里
            </p>
          </div>

          <!-- 连接符号 -->
          <div class="rounded-xl border border-slate-200 bg-white p-3.5 dark:border-slate-700 dark:bg-slate-800/60">
            <h4 class="mb-2.5 text-[11px] font-semibold uppercase tracking-wide text-slate-400 dark:text-slate-500">
              连接符号
            </h4>
            <el-tooltip
                v-for="s in SEPARATORS"
                :key="s.label"
                :content="s.tip"
                placement="top"
                :show-after="100"
            >
              <span
                  class="mb-1.5 mr-1.5 inline-flex h-8 min-w-8 select-none items-center justify-center rounded-lg border border-slate-200 bg-slate-50 px-2 text-[13px] font-semibold text-slate-600 transition hover:border-emerald-300 hover:bg-emerald-50 hover:text-emerald-700 dark:border-slate-700 dark:bg-slate-700/40 dark:text-slate-300 dark:hover:border-emerald-500/50 dark:hover:bg-emerald-500/15 dark:hover:text-emerald-400"
                  :class="
                  same
                    ? 'cursor-not-allowed opacity-45'
                    : 'cursor-grab touch-none active:cursor-grabbing'
                "
                  @pointerdown="!same && startDrag({ payload: 'conn_' + s.value, from: -1, label: s.label }, $event)"
                  @dblclick="!same && addConnector(s.value)"
              >
                {{ s.label }}
              </span>
            </el-tooltip>
            <p v-if="same" class="mt-1 text-[11px] text-slate-400 dark:text-slate-500">
              已开启统一分隔符，取消勾选后可单独设置
            </p>
          </div>

          <!-- 自定义连接符 -->
          <div class="rounded-xl border border-slate-200 bg-white p-3.5 dark:border-slate-700 dark:bg-slate-800/60">
            <h4 class="mb-2.5 text-[11px] font-semibold uppercase tracking-wide text-slate-400 dark:text-slate-500">
              自定义连接符
            </h4>
            <div class="flex gap-2">
              <el-input
                  v-model="customConnectorInput"
                  size="small"
                  placeholder="输入连接符..."
                  :disabled="same"
                  @keyup.enter="addCustomConnector"
              />
              <el-button size="small" type="primary" plain :disabled="same" @click="addCustomConnector">
                添加
              </el-button>
            </div>
            <div v-if="customConnectors.length > 0" class="mt-2 flex flex-wrap gap-1.5">
              <span
                  v-for="conn in customConnectors"
                  :key="conn"
                  class="cursor-grab select-none rounded-md bg-emerald-50 px-2 py-1 text-[11.5px] font-medium text-emerald-700 touch-none active:cursor-grabbing dark:bg-emerald-500/15 dark:text-emerald-400"
                  @pointerdown="!same && startDrag({ payload: 'conn_' + conn, from: -1, label: conn }, $event)"
                  @dblclick="!same && addConnector(conn)"
              >
                {{ conn }}
              </span>
            </div>
          </div>

        </div>

        <!-- 右侧：构建区 + 统一分隔符 + 预览 -->
        <div class="space-y-4">
          <!-- 构建区 -->
          <div class="overflow-hidden rounded-xl border border-slate-200 bg-white dark:border-slate-700 dark:bg-slate-800/60">
            <div class="flex flex-wrap items-center justify-between gap-2 border-b border-slate-100 px-4 py-3 dark:border-slate-700">
              <h3 class="text-sm font-semibold text-slate-800 dark:text-slate-100">命名构建区</h3>
              <div class="flex items-center gap-2">
                <span class="text-[11.5px] text-slate-400 dark:text-slate-500">拖入或双击左侧项，拖动可排序</span>
                <el-button size="small" plain @click="addConnector(activeSep)">+ 分隔符</el-button>
                <el-button
                    size="small"
                    type="danger"
                    plain
                    :disabled="segments.length === 0"
                    @click="clearSegments"
                >
                  清空
                </el-button>
              </div>
            </div>

            <div
                ref="dropArea"
                class="min-h-[132px] p-4 transition-colors"
                :class="dropIndex !== null ? 'bg-blue-50/60 dark:bg-blue-500/15' : ''"
            >
              <div
                  v-if="segments.length === 0"
                  class="flex flex-col items-center gap-2 rounded-xl border-2 border-dashed border-slate-200 py-9 text-[13px] text-slate-400 dark:border-slate-700 dark:text-slate-500"
                  :class="dropIndex !== null ? 'border-brand text-brand dark:text-brand-light' : ''"
              >
                <span class="text-2xl leading-none">⟱</span>
                <span>把左侧的字段、连接符拖到这里开始构建</span>
              </div>

              <div v-else class="flex flex-wrap items-center gap-2">
                <template v-for="(seg, index) in segments" :key="seg.id">
                  <!-- 落点指示条 -->
                  <span v-if="dropIndex === index" class="h-6 w-[3px] shrink-0 rounded-full bg-brand" />
                  <div
                      data-seg
                      class="flex cursor-grab select-none items-center gap-1.5 rounded-lg border px-2.5 py-1.5 text-[13px] font-medium transition active:cursor-grabbing touch-none"
                      :class="[
                      seg.type === 'keyword'
                        ? 'border-brand/40 bg-blue-50 text-brand dark:bg-blue-500/15 dark:text-brand-light'
                        : seg.type === 'connector'
                          ? 'border-emerald-200 bg-emerald-50 font-mono text-emerald-700 dark:border-emerald-500/40 dark:bg-emerald-500/15 dark:text-emerald-400'
                          : 'border-amber-200 bg-amber-50 text-amber-700 dark:border-amber-500/40 dark:bg-amber-500/15 dark:text-amber-400',
                      drag?.from === index ? 'opacity-40' : '',
                    ]"
                      @pointerdown="startDrag({ payload: '', from: index, label: segLabel(seg) }, $event)"
                  >
                    <span class="text-[11px] tracking-tighter opacity-40">⋮⋮</span>
                    <span>{{ segLabel(seg) }}</span>
                    <button
                        class="flex h-[18px] w-[18px] items-center justify-center rounded-full bg-black/10 p-0 text-[12px] leading-none opacity-60 transition hover:bg-black/20 hover:opacity-100 dark:bg-white/10 dark:hover:bg-white/20"
                        title="移除"
                        @pointerdown.stop="() => {}"
                        @click.stop="removeSegment(index)"
                    >
                      ×
                    </button>
                  </div>
                </template>
                <span
                    v-if="dropIndex === segments.length"
                    class="h-6 w-[3px] shrink-0 rounded-full bg-brand"
                />
              </div>
            </div>
          </div>

          <!-- 统一分隔符 -->
          <div class="flex flex-wrap items-center gap-3 rounded-xl border border-slate-200 bg-white px-4 py-3 dark:border-slate-700 dark:bg-slate-800/60">
            <el-checkbox v-model="same" size="large" class="!mr-0">
              <span class="font-semibold">统一分隔符</span>
            </el-checkbox>
            <template v-if="same">
              <span class="text-sm text-slate-500 dark:text-slate-400">为：</span>
              <el-radio-group v-model="sep" class="flex-wrap gap-y-2">
                <el-tooltip
                    v-for="s in SEP_OPTIONS"
                    :key="s.label"
                    :content="s.tip"
                    placement="top"
                    :show-after="100"
                >
                  <el-radio-button :value="s.value">{{ s.label }}</el-radio-button>
                </el-tooltip>
              </el-radio-group>
              <el-input
                  v-if="sep === CUSTOM"
                  v-model="customSep"
                  size="small"
                  class="!w-32"
                  placeholder="输入分隔符"
              />
            </template>
            <span v-else class="text-[11.5px] text-slate-400 dark:text-slate-500">
              构建区中的每个连接符可单独设置
            </span>
          </div>

          <!-- 预览 -->
          <div class="rounded-xl border border-slate-200 bg-white p-4 dark:border-slate-700 dark:bg-slate-800/60">
            <h4 class="mb-2.5 text-[11px] font-semibold uppercase tracking-wide text-slate-400 dark:text-slate-500">
              实时预览
            </h4>
            <div class="flex items-center gap-4">
              <div class="min-w-0 flex-1">
                <p class="mb-1 text-[10.5px] font-semibold uppercase tracking-wide text-slate-400 dark:text-slate-500">
                  命名格式
                </p>
                <p class="whitespace-pre break-all rounded-md border border-blue-100 bg-blue-50 px-2.5 py-1.5 font-mono text-[13px] font-semibold text-brand dark:border-blue-500/40 dark:bg-blue-500/15 dark:text-brand-light">
                  {{ formatText }}
                </p>
              </div>
              <span class="shrink-0 text-lg text-slate-300 dark:text-slate-600">→</span>
              <div class="min-w-0 flex-1">
                <p class="mb-1 text-[10.5px] font-semibold uppercase tracking-wide text-slate-400 dark:text-slate-500">
                  样例文件名
                </p>
                <p class="whitespace-pre break-all rounded-md border border-slate-200 bg-slate-50 px-2.5 py-1.5 font-mono text-[13px] text-slate-600 dark:border-slate-700 dark:bg-slate-700/40 dark:text-slate-300">
                  {{ exampleText }}
                </p>
              </div>
            </div>
            <p class="mt-2 text-[11.5px] text-slate-400 dark:text-slate-500">样例使用表格第一行数据生成</p>
          </div>
        </div>
      </div>

      <!-- 目录 -->
      <div class="mt-5 border-t border-slate-100 pt-4 dark:border-slate-700">
        <div
            ref="dirZone"
            class="rounded-xl border-2 border-dashed p-3 transition-colors"
            :class="dirDragging ? 'border-brand bg-blue-50 dark:bg-blue-500/15' : 'border-slate-200 dark:border-slate-700'"
            @dragover.prevent="dirDragging = true"
            @dragleave="dirDragging = false"
            @drop.prevent="onBrowserDrop"
        >
          <p class="mb-2 text-sm text-brand dark:text-brand-light">
            提示：把文件夹拖到虚线框内即可获取路径，也可以点击“浏览”选择或直接粘贴路径。
          </p>
          <div class="flex items-center gap-3">
            <el-input
                v-model="path"
                size="large"
                placeholder="请输入包含待改名文件的文件夹路径"
                clearable
                @keyup.enter="submit"
            />
            <el-button size="large" :icon="FolderOpened" @click="pickDir">浏览</el-button>
          </div>
        </div>
      </div>

      <div class="mt-6 flex items-center gap-3">
        <el-button size="large" :icon="ArrowLeft" @click="router.push('/keyword')">返回</el-button>
        <el-button
            type="primary"
            size="large"
            :loading="loading"
            :disabled="segments.length === 0"
            @click="submit"
        >
          提交
        </el-button>
      </div>
    </el-card>

    <!-- 拖拽时跟随光标的浮层 -->
    <div
        v-if="dragging"
        class="pointer-events-none fixed z-[9999] -translate-x-1/2 -translate-y-1/2 rounded-lg border border-brand bg-white px-2.5 py-1.5 text-[13px] font-medium text-brand shadow-lg dark:bg-slate-800 dark:text-brand-light"
        :style="{ left: ghost.x + 'px', top: ghost.y + 'px' }"
    >
      {{ ghost.label }}
    </div>
  </div>
</template>
