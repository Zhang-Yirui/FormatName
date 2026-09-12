<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { open } from '@tauri-apps/plugin-dialog'
import { ElMessage } from 'element-plus'
import { ArrowLeft, FolderOpened, MagicStick, RefreshRight } from '@element-plus/icons-vue'
import { submitExecute } from '@/api'
import { columns } from '@/store'
import type { ExecuteItem } from '@/types'
import {STEPS} from "@/constant";

const router = useRouter()

/** 默认连字符 */
const DEFAULT_SEP = '-'

/** 命名格式，奇偶交替：第 0/2/4... 项为字段，第 1/3/5... 项为分隔符 */
const picked = ref<ExecuteItem[]>(initPicked())
/** 是否统一分隔符 */
const same = ref(true)
/** 统一的分隔符 */
const sep = ref(DEFAULT_SEP)
/** 要改名的文件夹 */
const path = ref('')
const loading = ref(false)

function initPicked(): ExecuteItem[] {
  // 把关键字页选中的关键字全部加入命名格式，重复次数越少优先级越高
  const keywords = columns.value
      .map((col, index) => ({ col, index }))
      .filter(({ col }) => col.isKeyWord && col.display)
      .sort((a, b) => a.col.delta - b.col.delta)
      .map(({ index }) => index)

  if (keywords.length === 0) {
    return columns.value.length > 1 ? [0, DEFAULT_SEP, 1] : [0]
  }

  const items: ExecuteItem[] = []
  keywords.forEach((index, i) => {
    if (i > 0) items.push(DEFAULT_SEP)
    items.push(index)
  })
  return items
}

const SEPARATORS: { label: string; value: string }[] = [
  { label: '无', value: '' },
  { label: '-', value: '-' },
  { label: '空格', value: ' ' },
]

function applySep() {
  for (let i = 1; i < picked.value.length; i += 2) {
    picked.value[i] = sep.value
  }
}

watch(sep, () => {
  if (same.value) applySep()
})

watch(same, (val) => {
  if (val) applySep()
})

function increase() {
  if (picked.value.length > 12) {
    ElMessage.warning('哥，不至于！')
    return
  }
  picked.value.push(sep.value, 0)
}

function decrease() {
  if (picked.value.length > 3) {
    picked.value = picked.value.slice(0, -2)
  } else if (picked.value.length > 1) {
    picked.value = picked.value.slice(0, -1)
  }
}

/** 字段段显示用的自定义文本（字段序号不显示在输入框里） */
function customText(index: number): string {
  const value = picked.value[index]
  return typeof value === 'number' ? '' : (value ?? '')
}

function setCustomText(index: number, value: string) {
  picked.value[index] = value
}

function itemLabel(item: ExecuteItem): string {
  if (typeof item === 'number') return columns.value[item]?.key ?? String(item)
  return item ?? ''
}

function itemExample(item: ExecuteItem): string {
  if (typeof item === 'number') return columns.value[item]?.values[0] ?? ''
  return item ?? ''
}

const formatText = computed(
    () => picked.value.map(itemLabel).join('') + '.xxx',
)

const exampleText = computed(
    () => picked.value.map(itemExample).join('') + '.xxx',
)

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
      router.push('/analysis')
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
        <h1 class="page-title">{{STEPS[2].title}}</h1>
        <p class="page-desc">{{STEPS[2].hint}}</p>
      </div>
      <el-button text type="primary" :icon="RefreshRight" @click="router.push('/excel')">
        重新选择 Excel
      </el-button>
    </div>

    <el-card shadow="never" class="!rounded-2xl">
      <!-- 格式片段 -->
      <div class="flex flex-wrap items-stretch gap-3">
        <template v-for="(_item, index) in picked" :key="index">
          <!-- 字段 -->
          <el-card
              v-if="index % 2 === 0"
              shadow="never"
              class="w-56 !rounded-xl !border-brand/30"
              body-class="!p-3"
          >
            <template #header>
              <div class="flex items-center justify-between">
                <span class="text-xs font-semibold text-brand">
                  字段 {{ Math.floor(index / 2) + 1 }}
                </span>
                <el-tag size="small" type="info" effect="plain">表头</el-tag>
              </div>
            </template>
            <el-radio-group v-model="picked[index]" class="!flex !flex-col !items-start gap-1">
              <el-radio
                  v-for="(col, j) in columns"
                  :key="j"
                  :value="j"
                  :disabled="!col.display"
              >
                <span :class="{ 'text-slate-300': !col.display }">{{ col.key }}</span>
              </el-radio>
            </el-radio-group>
            <el-input
                :model-value="customText(index)"
                size="small"
                class="mt-2"
                placeholder="自定义文本"
                @update:model-value="(v: string) => setCustomText(index, v)"
            />
          </el-card>

          <!-- 分隔符 -->
          <el-card
              v-else-if="!same"
              shadow="never"
              class="w-40 !rounded-xl !border-slate-200"
              body-class="!p-3"
          >
            <template #header>
              <span class="text-xs font-semibold text-slate-500">分隔符</span>
            </template>
            <el-radio-group v-model="picked[index]" class="!flex !flex-col !items-start gap-1">
              <el-radio v-for="s in SEPARATORS" :key="s.label" :value="s.value">
                {{ s.label }}
              </el-radio>
            </el-radio-group>
            <el-input
                v-model="picked[index]"
                size="small"
                class="mt-2"
                placeholder="自定义"
            />
          </el-card>
        </template>

        <div class="flex items-center gap-2 self-center">
          <el-button :icon="MagicStick" circle title="增加一段" @click="increase" />
          <span class="text-sm text-slate-500">增加</span>
          <el-button circle title="减少一段" @click="decrease">－</el-button>
          <span class="text-sm text-slate-500">减少</span>
        </div>
      </div>

      <!-- 统一分隔符 -->
      <div class="mt-5 flex flex-wrap items-center gap-3 border-t border-slate-100 pt-4">
        <el-checkbox v-model="same" size="large" class="!mr-0">
          <span class="font-semibold">统一分隔符</span>
        </el-checkbox>
        <template v-if="same">
          <span class="text-sm text-slate-500">为：</span>
          <el-radio-group v-model="sep">
            <el-radio-button v-for="s in SEPARATORS" :key="s.label" :value="s.value">
              {{ s.label }}
            </el-radio-button>
          </el-radio-group>
          <span class="text-sm text-slate-500">或自定义</span>
          <el-input v-model="sep" size="small" class="!w-24" />
        </template>
      </div>

      <!-- 预览 -->
      <el-descriptions :column="1" border class="mt-5">
        <el-descriptions-item label="格式">
          <span class="whitespace-pre font-semibold text-brand">{{ formatText }}</span>
        </el-descriptions-item>
        <el-descriptions-item label="样例">
          <span class="whitespace-pre text-slate-600">{{ exampleText }}</span>
        </el-descriptions-item>
      </el-descriptions>

      <!-- 目录 -->
      <div class="mt-5 border-t border-slate-100 pt-4">
        <p class="mb-2 text-sm text-brand">
          提示：点击“浏览”选择文件夹，或粘贴文件夹路径。
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

      <div class="mt-6 flex items-center gap-3">
        <el-button size="large" :icon="ArrowLeft" @click="router.push('/keyword')">返回</el-button>
        <el-button type="primary" size="large" :loading="loading" @click="submit">提交</el-button>
      </div>
    </el-card>
  </div>
</template>
