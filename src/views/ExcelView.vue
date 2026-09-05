<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { open } from '@tauri-apps/plugin-dialog'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { ElMessage } from 'element-plus'
import { Document, FolderOpened, Right } from '@element-plus/icons-vue'
import { submitExcelPath } from '@/api'
import { columns, loadColumns, resetStep } from '@/store'

/** 允许的 Excel 扩展名 */
const EXCEL_EXT = ['xlsx', 'xlsm', 'xltx', 'xltm']

const router = useRouter()
const path = ref('')
const loading = ref(false)
const hasCache = ref(false)
/** 文件正被拖拽到窗口上 */
const dragging = ref(false)
let unlistenDragDrop: (() => void) | undefined

/** 校验并写入文件路径 */
function setPath(file: string) {
  const ext = file.slice(file.lastIndexOf('.') + 1).toLowerCase()
  if (!EXCEL_EXT.includes(ext)) {
    ElMessage.warning('请拖入 Excel 文件（.xlsx / .xlsm / .xltx / .xltm）')
    return
  }
  path.value = file
}

async function pickFile() {
  const selected = await open({
    multiple: false,
    directory: false,
    title: '选择 Excel 花名册',
    filters: [
      {
        name: 'Excel 表格',
        extensions: [...EXCEL_EXT],
      },
    ],
  })
  if (typeof selected === 'string' && selected) {
    path.value = selected
  }
}

/** 浏览器环境下的拖拽兜底（Tauri 窗口由原生拖拽事件处理） */
function onDrop(e: DragEvent) {
  dragging.value = false
  const file = e.dataTransfer?.files?.[0] as (File & { path?: string }) | undefined
  if (!file) return
  if (file.path) {
    setPath(file.path)
  } else {
    ElMessage.info('当前环境无法获取文件完整路径，请点击“浏览”按钮选择文件')
  }
}

async function submit() {
  if (!path.value.trim()) {
    ElMessage.warning('请先选择 Excel 文件')
    return
  }
  loading.value = true
  try {
    const res = await submitExcelPath(path.value.trim())
    if (res.code === 0) {
      ElMessage.success(res.msg)
      await loadColumns(true)
      // 换了新的花名册，后面的步骤需要重新走一遍
      resetStep(1)
      await router.push('/keyword')
    } else {
      ElMessage.error(res.msg)
    }
  } catch (e) {
    ElMessage.error(`读取失败：${e}`)
  } finally {
    loading.value = false
  }
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
        const file = payload.paths.find((p) =>
            EXCEL_EXT.some((ext) => p.toLowerCase().endsWith(`.${ext}`)),
        )
        if (file) setPath(file)
        else if (payload.paths.length > 0)
          ElMessage.warning('请拖入 Excel 文件（.xlsx / .xlsm / .xltx / .xltm）')
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
  <div class="mx-auto w-[90vw]">
    <el-card shadow="never" class="!rounded-2xl">
      <template #header>
        <div class="flex items-center gap-2 text-lg font-semibold text-brand">
          <el-icon><Document /></el-icon>
          <span>第一步：选择 Excel 花名册</span>
        </div>
      </template>

      <el-alert
          type="info"
          :closable="false"
          show-icon
          title="表格第一行为表头，第二行开始为数据，表头将作为关键字"
          class="mb-5"
      />

      <!-- 文件路径输入 + 浏览按钮（也可把文件拖到这里） -->
      <div
          class="path-row"
          :class="{ 'is-dragover': dragging }"
          @dragover.prevent="dragging = true"
          @dragleave.prevent="dragging = false"
          @drop.prevent="onDrop"
      >
        <el-input
            v-model="path"
            size="large"
            placeholder="请选择、粘贴 Excel 文件路径，或将文件拖拽到此处"
            clearable
            @keyup.enter="submit"
        />
        <el-button size="large" :icon="FolderOpened" @click="pickFile">浏览</el-button>
      </div>

      <p class="hint">
        也可以手动粘贴路径：按住 Shift 键，右键点击 Excel 文件，选择“复制为路径”。
      </p>

      <div class="actions">
        <el-button type="primary" size="large" :loading="loading" @click="submit">
          提交
        </el-button>
        <el-button
            v-if="hasCache"
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
.path-row {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.25rem;
  border: 1px dashed transparent;
  border-radius: 0.75rem;
  transition:
      background-color 0.2s ease,
      border-color 0.2s ease;
}

.path-row.is-dragover {
  border-color: var(--el-color-primary, #409eff);
  background-color: var(--el-color-primary-light-9, #ecf5ff);
}

.path-row .el-input {
  flex: 1;
  min-width: 0;
}

.path-row .el-button {
  flex-shrink: 0;
}

.hint {
  margin-top: 0.75rem;
  font-size: 12px;
  line-height: 1.5rem;
  color: #94a3b8;
  text-align: center;
}

.actions {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 1em;
  margin-top: 1.5rem;
}

.actions .el-button + .el-button {
  margin-left: 0;
}
</style>
