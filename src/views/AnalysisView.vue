<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import { ArrowLeft, CopyDocument, Refresh, RefreshRight } from '@element-plus/icons-vue'
import { backup, getExecute, recover, rename, rescan } from '@/api'
import { copyText } from '@/utils/clipboard'
import type { NamePair } from '@/types'
import {STEPS} from "@/constant";

const router = useRouter()

const list = ref<NamePair[]>([])
const newNames = ref<string[]>([])
const oldNames = ref<string[]>([])
const map = ref<number[]>([])
const flag = ref(0)
const loading = ref(false)
const refreshing = ref(false)

/** 已交（至少匹配到一次，包含多交的） */
const submitted = computed(() =>
    newNames.value.filter((_, i) => (map.value[i] ?? 0) > 0),
)

/** 没交（一次都没匹配到） */
const missing = computed(() =>
    newNames.value.filter((_, i) => (map.value[i] ?? 0) === 0),
)

/** 多交（同一个名字匹配到多次） */
const duplicated = computed(() =>
    newNames.value.filter((_, i) => (map.value[i] ?? 0) > 1),
)

/** 未知（没有匹配上任何关键字的旧文件） */
const unknown = computed(() => {
  const matched = new Set(list.value.map((i) => i.old))
  return oldNames.value.filter((name) => !matched.has(name))
})

async function load() {
  try {
    const res = await getExecute()
    list.value = res.list
    newNames.value = res.new
    oldNames.value = res.old
    map.value = res.map
    flag.value = res.flag
  } catch (e) {
    ElMessage.error(`获取数据失败：${e}`)
  }
}

/** 重新扫描目录：目录里的文件在程序外被改动后用它刷新分析结果 */
async function doRefresh() {
  // 已改名状态下刷新会改动磁盘：新出现的文件会被立即改名，丢失的文件会从对照表中移除
  if (flag.value !== 0) {
    try {
      await ElMessageBox.confirm(
          '将重新扫描目录：新放进来的文件会被立即改名，已经不在目录里的文件会从对照表中移除（无法再恢复）。是否继续？',
          '重新扫描',
          { type: 'warning', confirmButtonText: '继续扫描', cancelButtonText: '取消' },
      )
    } catch {
      return
    }
  }

  refreshing.value = true
  try {
    const res = await rescan()
    if (res.code !== 0) {
      ElMessage.error(res.msg)
      return
    }
    await load()
    ElMessage.success(res.msg)
  } catch (e) {
    ElMessage.error(`刷新失败：${e}`)
  } finally {
    refreshing.value = false
  }
}

async function copy(text: string) {
  try {
    await copyText(text)
    ElMessage.success('已复制到剪贴板')
  } catch (e) {
    ElMessage.error(`复制失败：${e instanceof Error ? e.message : e}`)
  }
}

async function doRename() {
  loading.value = true
  try {
    const res = await rename()
    if (res.code === 0) {
      ElMessage.success(res.msg)
      flag.value = 1
    } else {
      ElMessage.error(res.msg)
    }
  } catch (e) {
    ElMessage.error(`改名失败：${e}`)
  } finally {
    loading.value = false
  }
}

async function doRecover() {
  loading.value = true
  try {
    const res = await recover()
    if (res.code === 0) {
      ElMessage.success(res.msg)
      flag.value = 0
    } else {
      ElMessage.error(res.msg)
    }
  } catch (e) {
    ElMessage.error(`恢复失败：${e}`)
  } finally {
    loading.value = false
  }
}

async function doBackup() {
  loading.value = true
  try {
    const res = await backup()
    if (res.code !== 0) {
      ElMessage.error(res.msg)
      return
    }
    ElMessage.success(res.msg)
    await doRename()
  } catch (e) {
    ElMessage.error(`备份失败：${e}`)
  } finally {
    loading.value = false
  }
}

onMounted(load)
</script>

<template>
  <div class="page-view">
    <div class="page-header">
      <div>
        <h1 class="page-title">数据分析</h1>
        <p class="page-desc">{{STEPS[3].hint}}</p>
      </div>
      <el-tag :type="flag === 0 ? 'info' : 'success'" effect="dark" round>
        {{ flag === 0 ? '待改名' : '已改名' }}
      </el-tag>
    </div>

    <el-card shadow="never" class="!rounded-2xl dark:bg-slate-800 dark:border-slate-700">
      <div class="grid grid-cols-1 gap-5 lg:grid-cols-[minmax(0,1fr)_320px]">
        <!-- 新旧名字对照 -->
        <div>
          <h3 class="mb-2 text-sm font-semibold text-slate-600 dark:text-slate-400">
            新旧名字对照（{{ list.length }} 个文件）
          </h3>
          <el-table :data="list" border stripe max-height="420">
            <el-table-column prop="old" label="旧名字" min-width="200" show-overflow-tooltip>
              <template #default="{ row }">
                <span class="whitespace-pre">{{ row.old }}</span>
              </template>
            </el-table-column>
            <el-table-column prop="new" label="新名字" min-width="200" show-overflow-tooltip>
              <template #default="{ row }">
                <span class="whitespace-pre text-brand dark:text-brand-light">{{ row.new }}</span>
              </template>
            </el-table-column>
          </el-table>
        </div>

        <!-- 统计 -->
        <div class="space-y-4">
          <div>
            <h3
                class="mb-2 flex items-center gap-1 text-sm font-semibold text-slate-600 dark:text-slate-400"
                title="至少匹配到一次，包含多交的"
            >
              已交
              <el-tag size="small" type="success" effect="dark" round>
                {{ submitted.length }}
              </el-tag>
            </h3>
            <el-scrollbar max-height="130">
              <ul class="space-y-1">
                <li
                    v-for="name in submitted"
                    :key="name"
                    class="flex cursor-pointer items-center justify-between rounded bg-green-50 px-3 py-1 text-xs text-green-700 transition hover:bg-green-100 dark:bg-green-500/10 dark:text-green-400 dark:hover:bg-green-500/20"
                    @click="copy(name)"
                >
                  <span class="whitespace-pre">{{ name }}</span>
                  <el-icon><CopyDocument /></el-icon>
                </li>
                <li v-if="submitted.length === 0" class="px-3 text-xs text-slate-400 dark:text-slate-500">无</li>
              </ul>
            </el-scrollbar>
          </div>

          <div>
            <h3 class="mb-2 flex items-center gap-1 text-sm font-semibold text-slate-600 dark:text-slate-400">
              没交
              <el-tag size="small" type="danger" effect="dark" round>
                {{ missing.length }}
              </el-tag>
            </h3>
            <el-scrollbar max-height="130">
              <ul class="space-y-1">
                <li
                    v-for="name in missing"
                    :key="name"
                    class="flex cursor-pointer items-center justify-between rounded bg-red-50 px-3 py-1 text-xs text-red-700 transition hover:bg-red-100 dark:bg-red-500/10 dark:text-red-400 dark:hover:bg-red-500/20"
                    @click="copy(name)"
                >
                  <span class="whitespace-pre">{{ name }}</span>
                  <el-icon><CopyDocument /></el-icon>
                </li>
                <li v-if="missing.length === 0" class="px-3 text-xs text-slate-400 dark:text-slate-500">无</li>
              </ul>
            </el-scrollbar>
          </div>

          <div>
            <h3 class="mb-2 flex items-center gap-1 text-sm font-semibold text-slate-600 dark:text-slate-400">
              多交
              <el-tag size="small" type="warning" effect="dark" round>
                {{ duplicated.length }}
              </el-tag>
            </h3>
            <el-scrollbar max-height="130">
              <ul class="space-y-1">
                <li
                    v-for="name in duplicated"
                    :key="name"
                    class="flex cursor-pointer items-center justify-between rounded bg-amber-50 px-3 py-1 text-xs text-amber-700 transition hover:bg-amber-100 dark:bg-amber-500/10 dark:text-amber-400 dark:hover:bg-amber-500/20"
                    @click="copy(name)"
                >
                  <span class="whitespace-pre">{{ name }}</span>
                  <el-icon><CopyDocument /></el-icon>
                </li>
                <li v-if="duplicated.length === 0" class="px-3 text-xs text-slate-400 dark:text-slate-500">无</li>
              </ul>
            </el-scrollbar>
          </div>

          <div>
            <h3 class="mb-2 flex items-center gap-1 text-sm font-semibold text-slate-600 dark:text-slate-400">
              未知
              <el-tag size="small" type="info" effect="dark" round>
                {{ unknown.length }}
              </el-tag>
            </h3>
            <el-scrollbar max-height="130">
              <ul class="space-y-1">
                <li
                    v-for="name in unknown"
                    :key="name"
                    class="flex cursor-pointer items-center justify-between rounded bg-slate-100 px-3 py-1 text-xs text-slate-600 transition hover:bg-slate-200 dark:bg-slate-700/50 dark:text-slate-300 dark:hover:bg-slate-700"
                    @click="copy(name)"
                >
                  <span class="whitespace-pre">{{ name }}</span>
                  <el-icon><CopyDocument /></el-icon>
                </li>
                <li v-if="unknown.length === 0" class="px-3 text-xs text-slate-400 dark:text-slate-500">无</li>
              </ul>
            </el-scrollbar>
          </div>

          <p class="text-xs text-slate-400 dark:text-slate-500">点击列表内容即可复制到剪贴板</p>
        </div>
      </div>

      <div class="mt-6 flex flex-wrap items-center gap-3 border-t border-slate-100 pt-4 dark:border-slate-700">
        <el-button size="large" :icon="ArrowLeft" @click="router.push('/format')">返回</el-button>
        <el-button v-if="flag === 0" type="primary" size="large" :loading="loading" @click="doRename">
          改名
        </el-button>
        <el-button v-else type="danger" size="large" :icon="Refresh" :loading="loading" @click="doRecover">
          恢复
        </el-button>
        <el-button v-if="flag === 0" size="large" :loading="loading" @click="doBackup">
          备份并改名
        </el-button>
        <el-button
            size="large"
            text
            :icon="RefreshRight"
            :loading="refreshing"
            title="重新扫描目录并重新计算匹配结果（改名后刷新会立即处理新放入的文件）"
            @click="doRefresh"
        >
          刷新
        </el-button>
      </div>
    </el-card>
  </div>
</template>
