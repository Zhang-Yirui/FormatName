<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import { RefreshLeft } from '@element-plus/icons-vue'
import { openUrl } from '@tauri-apps/plugin-opener'
import {
  IconHelpHexagon,
  IconInfoHexagon,
  IconMoon,
  IconSun,
} from '@tabler/icons-vue'
import { SolarDay } from 'tyme4ts'
import { clearData, getAppInfo } from '@/api'
import {EXCEL_EXT, STEPS, TEXT_EXT} from '@/constant'
import { columns, maxStep, resetColumns } from '@/store'
import type { AppInfoResp } from '@/types'
import { tablerIcon } from '@/utils/tablerIcon'
import { isDark, toggleDark } from '@/utils/theme'

const aboutVisible = ref(false)
const appInfo = ref<AppInfoResp>({
  name: 'FormatName',
  version: '0.0.0',
  description: 'FormatName 格式化命名',
  authors: ['静凇'],
  homepage: '#',
  license: 'Apache-2.0',
  repository: '#',
})

async function openLink(url: string) {
  try {
    await openUrl(url)
  } catch {
    window.open(url, '_blank')
  }
}

/* ---------------- 头部日期时间（含农历） ---------------- */

/** 每秒刷新，用于显示时:分:秒 */
const clock = ref(new Date())
/** 当天 0 点后的日期对象，跨天后才更新，避免每秒重算农历 */
const today = ref(new Date())
let timer: number | undefined

function sameDay(a: Date, b: Date): boolean {
  return (
      a.getFullYear() === b.getFullYear() &&
      a.getMonth() === b.getMonth() &&
      a.getDate() === b.getDate()
  )
}

function tick() {
  const now = new Date()
  clock.value = now
  if (!sameDay(now, today.value)) today.value = now
}

/** 当天对应的公历日 */
const solarDay = computed(() =>
    SolarDay.fromYmd(today.value.getFullYear(), today.value.getMonth() + 1, today.value.getDate()),
)

/** 当天对应的农历日 */
const lunarDay = computed(() => solarDay.value.getLunarDay())

/** 时:分:秒 */
const timeText = computed(() => {
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${pad(clock.value.getHours())}:${pad(clock.value.getMinutes())}:${pad(clock.value.getSeconds())}`
})

/** 公历日期 + 星期：2026年9月15日 星期二 */
const solarText = computed(() =>
    `${solarDay.value.toString()} 星期${solarDay.value.getWeek().getName()}`,
)

/** 农历日期：农历八月初五 */
const lunarText = computed(() => `农历${lunarDay.value.getLunarMonth().getName()}${lunarDay.value.getName()}`)

/** 农历年干支生肖：丙午[马]年 */
const lunarYearText = computed(() => {
  const cycle = lunarDay.value.getYearSixtyCycle()
  return `${cycle.getName()}[${cycle.getEarthBranch().getZodiac().getName()}]年`
})

onMounted(async () => {
  tick()
  timer = window.setInterval(tick, 1000)
  try {
    appInfo.value = await getAppInfo()
    appInfo.value.version = `V${appInfo.value.version}`
  } catch {
    appInfo.value.version = 'Unknown'
  }
})

onUnmounted(() => {
  if (timer !== undefined) clearInterval(timer)
})

/* ---------------- 左侧步骤导航 ---------------- */

const route = useRoute()
const router = useRouter()

const activeStep = computed(() => Number(route.meta.step ?? 0))
const keywordCount = computed(() => columns.value.filter((c) => c.isKeyWord).length)
const hint = computed(() => STEPS[activeStep.value].hint ?? '')

/** 是否已解锁：只有到达过或更早的步骤可以点击跳转 */
function unlocked(index: number): boolean {
  return index <= maxStep.value
}

/** 步骤状态：cur 当前 / done 已完成 / locked 未解锁 / '' 未到达 */
type StepState = 'cur' | 'done' | 'locked' | ''

function stateOf(index: number): StepState {
  if (index === activeStep.value) return 'cur'
  if (index < maxStep.value) return 'done'
  if (!unlocked(index)) return 'locked'
  return ''
}

/** 步骤行：仅锁定态禁用交互；当前步骤整行略微放大 */
function rowClass(index: number): string {
  if (stateOf(index) === 'locked') return 'cursor-not-allowed opacity-[0.42]'
  return stateOf(index) === 'cur' ? 'cursor-default step-cur' : 'cursor-pointer'
}

/** 步骤序号圆点 */
function dotClass(index: number): string {
  const base =
      'flex shrink-0 items-center justify-center rounded-full border-[1.8px] font-bold transition-all duration-200'
  switch (stateOf(index)) {
    case 'done':
      return `${base} h-[26px] w-[26px] border-green-300 bg-emerald-50 text-xs text-green-600 dark:border-green-500/60 dark:bg-emerald-500/15 dark:text-green-400`
    case 'cur':
      return `${base} h-[29px] w-[29px] border-brand bg-brand text-[13px] text-white shadow-[0_0_0_5px_rgba(64,158,255,0.12)] dark:shadow-[0_0_0_5px_rgba(64,158,255,0.22)]`
    default:
      return `${base} h-[26px] w-[26px] border-slate-300 bg-white text-xs text-slate-400 dark:border-slate-600 dark:bg-slate-800 dark:text-slate-500`
  }
}

/** 步骤名称框 */
function labelClass(index: number): string {
  const base =
      'flex min-w-0 flex-1 flex-col gap-px rounded-lg border shadow-[0_1px_1.5px_rgba(0,0,0,0.025)] transition-all duration-200'
  switch (stateOf(index)) {
    case 'cur':
      return `${base} border-blue-200 bg-blue-50 px-[13px] py-[9px] dark:border-blue-500/40 dark:bg-blue-500/15`
    case 'locked':
      return `${base} border-slate-200 bg-white px-3 py-2 dark:border-slate-700 dark:bg-slate-800/80`
    default:
      return `${base} border-slate-200 bg-white px-3 py-2 group-hover:border-slate-300 group-hover:bg-slate-100 dark:border-slate-700 dark:bg-slate-800/80 dark:group-hover:border-slate-600 dark:group-hover:bg-slate-700`
  }
}

/** 步骤标题 */
function labelNameClass(index: number): string {
  switch (stateOf(index)) {
    case 'cur':
      return 'text-[14px] font-semibold text-brand dark:text-brand-light'
    case 'done':
      return 'text-[13px] font-medium text-slate-500 dark:text-slate-400'
    default:
      return 'text-[13px] font-medium text-slate-700 dark:text-slate-300'
  }
}

/** 步骤描述 */
function labelDescClass(index: number): string {
  return stateOf(index) === 'cur'
      ? 'text-[11.5px] text-blue-300 dark:text-blue-300/85'
      : 'text-[11px] text-slate-400 dark:text-slate-500'
}

/** 连接线左边距：跟随相邻圆点半径，保证与放大后的圆点居中对齐 */
function connectorPad(index: number): string {
  const center = (i: number) => (stateOf(i) === 'cur' ? 14.5 : 13)
  return `${(center(index) + center(index + 1)) / 2 - 0.75}px`
}

async function go(index: number) {
  if (index === activeStep.value) return
  if (!unlocked(index)) {
    ElMessage.warning('请先完成当前步骤，点击“提交”后才能进入后面的步骤')
    return
  }
  await router.push(STEPS[index]!.path)
}

/** 清空已导入的数据，回到第一步 */
async function reset() {
  try {
    await ElMessageBox.confirm(
        '将清空已导入的花名册与命名配置，回到第一步。是否继续？',
        '重置',
        { type: 'warning', confirmButtonText: '重置', cancelButtonText: '取消' },
    )
  } catch {
    return
  }
  try {
    await clearData()
  } catch {
    /* 非 Tauri 环境（浏览器调试）忽略 */
  }
  resetColumns()
  await router.push('/excel')
  ElMessage.success('已重置，请重新选择 Excel')
}
</script>

<template>
  <div class="h-screen">
    <el-container class="h-full">
      <!-- 左侧：步骤导航 -->
      <el-aside class="flex w-60 shrink-0 flex-col overflow-hidden border-r border-slate-200 bg-slate-50 transition-colors duration-200 dark:border-slate-800 dark:bg-slate-900">
        <!-- 品牌 -->
        <el-header class="flex items-center gap-3 border-b border-slate-100 bg-white px-4 py-[18px] transition-colors duration-200 dark:border-slate-800 dark:bg-slate-800">
          <el-image
              class="block h-[34px] w-[34px] shrink-0 rounded-lg object-contain"
              src="/favicon.svg"
              :alt="appInfo.name"
          />
          <div class="flex min-w-0 flex-col gap-px">
            <span class="text-sm font-semibold text-slate-800 dark:text-slate-100">{{appInfo.name}}</span>
            <span class="text-[10.5px] text-slate-400 dark:text-slate-500">批量重命名工具</span>
          </div>
        </el-header>

        <!-- 步骤树 -->
        <nav class="flex-1 overflow-y-auto px-3 pb-3 pl-4 pt-5">
          <template v-for="(step, index) in STEPS" :key="step.path">
            <div
                class="group flex select-none items-center gap-2 transition-all duration-200"
                :class="rowClass(index)"
                @click="go(index)"
            >
              <span :class="dotClass(index)">{{ index + 1 }}</span>
              <span :class="labelClass(index)">
                <span :class="labelNameClass(index)">{{ step.title }}</span>
                <span :class="labelDescClass(index)">{{ step.desc }}</span>
              </span>
            </div>
            <div v-if="index < STEPS.length - 1" class="h-[10px]" :style="{ paddingLeft: connectorPad(index) }">
              <i
                  class="block h-full w-[1.5px] transition-colors duration-300"
                  :class="index < maxStep ? 'bg-blue-300 dark:bg-blue-500/60' : 'bg-slate-200 dark:bg-slate-700'"
              />
            </div>
          </template>

          <p class="mb-0 mt-[18px] rounded-lg border border-slate-100 bg-white px-3 py-2.5 text-[11.5px] leading-[1.6] text-slate-500 transition-colors duration-200 dark:border-slate-700 dark:bg-slate-800 dark:text-slate-400">
            {{ hint }}
          </p>
        </nav>

        <!-- 底部 -->
        <el-footer class="flex items-center gap-3 border-t border-slate-100 bg-white px-4 py-3 text-[11px] text-slate-400 transition-colors duration-200 dark:border-slate-800 dark:bg-slate-800 dark:text-slate-500">
          <span><b class="mr-0.5 text-[13px] font-semibold text-slate-800 dark:text-slate-100">{{ columns.length }}</b> 列</span>
          <span><b class="mr-0.5 text-[13px] font-semibold text-slate-800 dark:text-slate-100">{{ keywordCount }}</b> 关键字</span>
          <el-button class="ml-auto shrink-0" size="small" :icon="RefreshLeft" @click="reset">重置</el-button>
        </el-footer>
      </el-aside>

      <el-container class="flex h-full min-w-0 flex-col">
        <!-- 左上： -->
        <el-header class="flex h-18 shrink-0 items-center justify-between border-b border-slate-200 bg-white px-6 py-19 transition-colors duration-200 dark:border-slate-800 dark:bg-slate-800">
          <div class="mr-auto flex min-w-0 items-center gap-3">
            <span class="fn-header-title text-[30px] font-bold tabular-nums tracking-wide text-slate-800 dark:text-slate-100">
              {{ timeText }}
            </span>
            <div class="flex min-w-0 flex-col gap-px leading-tight">
              <span class="truncate text-[13px] font-medium text-slate-700 dark:text-slate-300">{{ solarText }}</span>
              <span class="truncate text-[11px] text-slate-400 dark:text-slate-500">
                {{ lunarText }} · {{ lunarYearText }}
              </span>
            </div>
          </div>
          <!-- 右上：工具按钮 -->
          <div class="header-actions ml-4 shrink-0">
            <el-tooltip :content="isDark ? '切换亮色模式' : '切换暗色模式'" placement="bottom">
              <el-button :icon="isDark ? tablerIcon(IconMoon) : tablerIcon(IconSun)" circle @click="toggleDark()" />
            </el-tooltip>
            <el-tooltip content="使用手册" placement="bottom">
              <el-button :icon="tablerIcon(IconHelpHexagon)" circle @click="openLink(appInfo.homepage)" />
            </el-tooltip>
            <el-tooltip content="关于" placement="bottom">
              <el-button :icon="tablerIcon(IconInfoHexagon)" circle @click="aboutVisible = true" />
            </el-tooltip>
          </div>
        </el-header>

        <!-- 右侧：页面路由 -->
        <el-main class="min-h-0 flex-1 overflow-auto bg-slate-50 px-8 py-6 transition-colors duration-200 max-[1100px]:px-5 max-[1100px]:py-5 dark:bg-slate-900">
          <div class="mx-auto flex min-h-full w-full max-w-[1180px] flex-col">
            <router-view v-slot="{ Component }">
              <transition name="page" mode="out-in">
                <component :is="Component" />
              </transition>
            </router-view>
          </div>
        </el-main>
      </el-container>
    </el-container>

    <el-dialog v-model="aboutVisible" :title="`关于 ${appInfo.name}`" width="500px">
      <div class="space-y-2 text-sm leading-6 text-slate-600 dark:text-slate-300">
        <p style="text-align: center">
          <b style="font-size: 16px">{{ appInfo.description }}</b>
        </p>
        <p>
          <b>功能</b>：通过 Excel 花名册名单，对上交的文件进行固定格式的批量重命名，可以自定义文本和自定义分隔符号，支持的表格格式：
          {{[...EXCEL_EXT, ...TEXT_EXT].map(ext => `${ext}`).join('、')}}。
        </p>
        <p><b>构建</b>：使用 <b>Tauri + Rust + Vue</b> 实现，无需额外安装运行环境，双击即用。</p>
        <p class="text-xs text-slate-400 dark:text-slate-500" style="display: flex; gap: 5em">
          <span><b>版本</b>：{{ appInfo.version }}</span>
          <span><b>作者</b>：{{ appInfo.authors.join(' · ') }}</span>
          <span><b>开源协议</b>：{{ appInfo.license }}</span>
        </p>
        <p class="text-xs text-slate-400 dark:text-slate-500">
          <span><b>注意</b>：请不要把本程序和要改名的文件放在同一个文件夹里。</span>
        </p>
      </div>
    </el-dialog>
  </div>
</template>

<style scoped>
/* 布局尺寸全部交给 Tailwind 工具类，这里只处理组件库内部的细节：
   相邻按钮的默认左边距与图标大小（工具类无法覆盖组件内部样式） */
.header-actions :deep(.el-button) + .el-button {
  margin-left: 15px;
}

.header-actions :deep(.el-button svg) {
  transform: scale(1.6);
  transform-origin: center;
}

/* 当前步骤整行略微放大（工具类无法同时表达 transform 与原点） */
.step-cur {
  transform: scale(1.02);
  transform-origin: left center;
}

</style>

<style>
/* 路由切换过渡（作用在各步骤页的根元素上） */
.page-enter-active,
.page-leave-active {
  transition: opacity 0.18s ease, transform 0.18s ease;
}
.page-enter-from {
  opacity: 0;
  transform: translateY(6px);
}
.page-leave-to {
  opacity: 0;
  transform: translateY(-6px);
}
</style>
