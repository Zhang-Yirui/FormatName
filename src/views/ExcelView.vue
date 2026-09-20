<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { open } from '@tauri-apps/plugin-dialog'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { ElMessage } from 'element-plus'
import { FolderOpened, Right } from '@element-plus/icons-vue'
import { submitExcelPath, submitTableText } from '@/api'
import {STEPS} from '@/constant'
import { columns, loadColumns, resetStep } from '@/store'

/** Excel 走后端按路径读取 */
const EXCEL_EXT = ['xlsx', 'xlsm', 'xltx', 'xltm']
/** 文本表格在后端按 CSV 解析 */
const TEXT_EXT = ['csv', 'txt']
const ALL_EXT = [...EXCEL_EXT, ...TEXT_EXT]

const router = useRouter()
const loading = ref(false)
/** 文件正被拖拽到窗口上 */
const dragging = ref(false)
/** 当前导入的来源（文件名） */
const sourceName = ref('')
/** 本次进入页面后是否重新导入过数据 */
const imported = ref(false)
/** 进入页面时后端是否已经有数据 */
const hasCache = ref(false)
/** 浏览器环境下的文件选择框（Tauri 文件对话框的兜底） */
const fileInput = ref<HTMLInputElement | null>(null)

let unlistenDragDrop: (() => void) | undefined
/** 同一次拖拽会被原生事件和浏览器事件各触发一次，用它去重 */
let lastDropAt = 0

function duplicatedDrop(): boolean {
  const now = Date.now()
  if (now - lastDropAt < 1000) return true
  lastDropAt = now
  return false
}

const rowCount = computed(() => columns.value[0]?.values.length ?? 0)
const hasData = computed(() => columns.value.length > 0 && rowCount.value > 0)

/** 列数据转换成 el-table 需要的行数据 */
const rows = computed(() =>
    Array.from({ length: rowCount.value }, (_, i) => {
      const row: Record<string, string> = {}
      columns.value.forEach((col, j) => {
        row[String(j)] = col.values[i] ?? ''
      })
      return row
    }),
)

function extOf(name: string): string {
  return name.slice(name.lastIndexOf('.') + 1).toLowerCase()
}

/** 导入成功后刷新列数据，并让后面的步骤重新走一遍 */
async function afterImport(name: string) {
  sourceName.value = name
  imported.value = true
  columns.value = await loadColumns(true)
  resetStep(1)
}

/** 按路径导入（Excel / CSV 都由后端解析） */
async function importPath(file: string) {
  const ext = extOf(file)
  if (!ALL_EXT.includes(ext)) {
    ElMessage.warning('请选择 Excel（.xlsx / .xlsm / .xltx / .xltm）或 CSV（.csv）文件')
    return
  }
  loading.value = true
  try {
    const res = await submitExcelPath(file)
    if (res.code !== 0) {
      ElMessage.error(res.msg)
      return
    }
    await afterImport(file.split(/[\\/]/).pop() ?? file)
    ElMessage.success(res.msg)
  } catch (e) {
    ElMessage.error(`读取失败：${e}`)
  } finally {
    loading.value = false
  }
}

/** 导入一段文本（浏览器环境拿不到文件完整路径时的兜底） */
async function importText(name: string, text: string) {
  if (!text.trim()) {
    ElMessage.warning('文件内容为空，请重新选择表格文件')
    return
  }
  loading.value = true
  try {
    const res = await submitTableText(text)
    if (res.code !== 0) {
      ElMessage.error(res.msg)
      return
    }
    await afterImport(name)
    ElMessage.success(res.msg)
  } catch (e) {
    ElMessage.error(`解析失败：${e}`)
  } finally {
    loading.value = false
  }
}

/** 处理一个 File（拖拽或文件选择框） */
async function processFile(file: File) {
  const path = (file as File & { path?: string }).path
  // 拿到完整路径时交给后端读取，Excel 只能这样解析
  if (path) {
    await importPath(path)
    return
  }
  if (TEXT_EXT.includes(extOf(file.name))) {
    await importText(file.name, await file.text())
    return
  }
  ElMessage.info('当前环境无法获取文件完整路径，请点击“选择文件”按钮选择文件')
}

async function pickFile() {
  let selected: string | string[] | null
  try {
    selected = await open({
      multiple: false,
      directory: false,
      title: '选择花名册',
      filters: [
        { name: '表格文件', extensions: [...ALL_EXT] },
        { name: 'Excel 表格', extensions: [...EXCEL_EXT] },
        { name: '文本表格', extensions: [...TEXT_EXT] },
      ],
    })
  } catch {
    // 非 Tauri 环境（浏览器调试）退回浏览器的文件选择框
    fileInput.value?.click()
    return
  }
  if (typeof selected === 'string' && selected) await importPath(selected)
}

function onFileSelect(e: Event) {
  const input = e.target as HTMLInputElement
  const file = input.files?.[0]
  if (file) void processFile(file)
  // 清空 value，这样重复选择同一个文件也会触发 change
  input.value = ''
}

/** 浏览器环境下的拖拽兜底（Tauri 窗口由原生拖拽事件处理） */
function onDrop(e: DragEvent) {
  dragging.value = false
  const file = e.dataTransfer?.files?.[0]
  if (file && !duplicatedDrop()) void processFile(file)
}

async function next() {
  if (!hasData.value) return
  await router.push('/keyword')
}

onMounted(async () => {
  const data = await loadColumns()
  hasCache.value = data.length > 0
  if (data.length > 0) columns.value = data

  // 监听 Tauri 原生文件拖拽事件（可以拿到文件的完整路径）
  try {
    unlistenDragDrop = await getCurrentWebview().onDragDropEvent(({ payload }) => {
      if (payload.type === 'enter' || payload.type === 'over') {
        dragging.value = true
      } else if (payload.type === 'drop') {
        dragging.value = false
        const file = payload.paths.find((p) => ALL_EXT.includes(extOf(p)))
        if (file && !duplicatedDrop()) void importPath(file)
        else if (!file && payload.paths.length > 0 && !duplicatedDrop())
          ElMessage.warning('请选择 Excel（.xlsx / .xlsm / .xltx / .xltm）或 CSV（.csv）文件')
      } else {
        dragging.value = false
      }
    })
  } catch {
    /* 非 Tauri 环境（浏览器调试）忽略原生拖拽 */
  }
})

onUnmounted(() => unlistenDragDrop?.())
</script>

<template>
  <div class="page-view">
    <div class="page-header">
      <div>
        <h1 class="page-title">{{STEPS[0].desc}}</h1>
        <p class="page-desc">{{STEPS[0].hint}}</p>
      </div>
    </div>

    <el-card v-loading="loading" shadow="never" class="!rounded-2xl dark:bg-slate-800 dark:border-slate-700">
      <el-alert
          type="info"
          :closable="false"
          show-icon
          title="表格第一行为表头，第二行开始为数据，表头将作为关键字"
          class="mb-4"
      />

      <!-- 导入文件 -->
      <div
          class="drop-zone"
          :class="{ 'is-dragover': dragging }"
          @dragover.prevent="dragging = true"
          @dragleave.prevent="dragging = false"
          @drop.prevent="onDrop"
      >
        <div class="drop-icon">📄</div>
        <p class="drop-text">将文件拖拽到此处，或点击下方选择文件</p>
        <p class="drop-hint">支持 .xlsx / .xlsm / .xltx / .xltm / .csv</p>
        <el-button :icon="FolderOpened" @click="pickFile">选择文件</el-button>
        <input
            ref="fileInput"
            type="file"
            class="hidden-input"
            :accept="`.${ALL_EXT.join(',.')}`"
            @change="onFileSelect"
        />
      </div>

      <!-- 表格数据（导入后立即显示） -->
      <template v-if="hasData">
        <div class="mt-5 flex items-center justify-between">
          <span class="text-[13px] font-semibold text-slate-700 dark:text-slate-300">
            表格数据
            <span v-if="sourceName" class="ml-2 text-[11.5px] font-normal text-slate-400 dark:text-slate-500">
              {{ sourceName }}
            </span>
          </span>
          <span class="text-[11.5px] text-slate-400 dark:text-slate-500">
            共 {{ columns.length }} 列 · {{ rowCount }} 行
          </span>
        </div>
        <el-table
            :data="rows"
            border
            stripe
            size="small"
            max-height="380"
            class="mt-2"
        >
          <el-table-column
              v-for="(col, index) in columns"
              :key="index"
              :prop="String(index)"
              :label="col.key"
              align="center"
              header-align="center"
              min-width="120"
              show-overflow-tooltip
          />
        </el-table>
      </template>
      <p v-else class="mt-5 text-center text-[13px] text-slate-400 dark:text-slate-500">尚未导入表格数据，请先选择或拖入表格文件</p>

      <div class="mt-5 flex items-center gap-3">
        <el-button type="primary" size="large" :icon="Right" :disabled="!hasData" @click="next">
          下一步 · 选择关键字
        </el-button>
        <el-button
            v-if="hasCache && !imported"
            type="success"
            size="large"
            :icon="Right"
            @click="router.push('/format')"
        >
          继续使用上次的数据
        </el-button>
      </div>
    </el-card>
  </div>
</template>

<style scoped>
/* 拖拽区 */
.drop-zone {
  margin-top: 16px;
  padding: 36px 24px;
  border: 2px dashed #d0d5dd;
  border-radius: 12px;
  background: #fafbfc;
  text-align: center;
  transition:
      border-color 0.2s ease,
      background-color 0.2s ease;
}

.drop-zone.is-dragover {
  border-color: var(--el-color-primary, #409eff);
  background: var(--el-color-primary-light-9, #ecf5ff);
}

.drop-icon {
  margin-bottom: 8px;
  font-size: 32px;
}

.drop-text {
  margin: 0 0 6px;
  font-size: 14px;
  font-weight: 500;
  color: #384252;
}

.drop-hint {
  margin: 0 0 14px;
  font-size: 13px;
  color: #949aab;
}

/* 暗色模式：拖拽区改为深色底 + 亮边框 */
html.dark .drop-zone {
  border-color: #475569;
  background: #1e293b;
}

html.dark .drop-zone.is-dragover {
  border-color: var(--el-color-primary, #409eff);
  background: rgba(64, 158, 255, 0.16);
}

html.dark .drop-text {
  color: #cbd5e1;
}

html.dark .drop-hint {
  color: #94a3b8;
}

.hidden-input {
  display: none;
}

</style>
