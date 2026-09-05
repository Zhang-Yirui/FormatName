<script setup lang="ts">
import {computed, onMounted, ref} from 'vue'
import {useRoute, useRouter} from 'vue-router'
import {openUrl} from '@tauri-apps/plugin-opener'
import {ElMessage} from 'element-plus'
import {getAppInfo} from '@/api'
import {
  IconHelpHexagon, IconHexagonNumber1, IconHexagonNumber2, IconHexagonNumber3, IconHexagonNumber4, IconInfoHexagon,
  IconMoon,
  IconSun,
} from '@tabler/icons-vue';
import {AppInfoResp} from "@/types";
import {tablerIcon} from '@/utils/tablerIcon'
import {maxStep} from '@/store'

const route = useRoute()
const router = useRouter()
const dark = ref(false)
const appInfo = ref<AppInfoResp>({
  name: "FormatName",
  version: "0.0.0",
  description: "FormatName 格式化命名",
  authors: ["静凇"],
  homepage: "#",
  license: "MIT",
  repository: "#"
})
const aboutVisible = ref(false)

const activeStep = computed(() => Number(route.meta.step ?? 0))

/** 步骤条配置：顺序即步骤序号（0 起） */
const steps = [
  {title: '选择 Excel', description: '导入花名册', icon: IconHexagonNumber1, path: '/excel'},
  {title: '选择关键字', description: '点击表头切换', icon: IconHexagonNumber2, path: '/keyword'},
  {title: '设置命名格式', description: '自定义分隔符', icon: IconHexagonNumber3, path: '/format'},
  {title: '数据分析', description: '改名 / 恢复 / 备份', icon: IconHexagonNumber4, path: '/analysis'},
]

/** 是否已解锁：只有到达过或更早的步骤可以点击跳转 */
function unlocked(index: number): boolean {
  return index <= maxStep.value
}

/** 点击步骤：之前的步骤直接跳转，之后的步骤提示先提交当前页面 */
function onStepClick(index: number) {
  if (index === activeStep.value) return
  if (!unlocked(index)) {
    ElMessage.warning('请先完成当前步骤，点击“提交”后才能进入后面的步骤')
    return
  }
  router.push(steps[index].path)
}

async function openLink(url: string) {
  try {
    await openUrl(url)
  } catch {
    window.open(url, '_blank')
  }
}

onMounted(async () => {
  try {
    appInfo.value = await getAppInfo()
    appInfo.value.version = `V${appInfo.value.version}`
    console.log(appInfo)
  } catch {
    appInfo.value.version = 'Unknown'
  }
})
</script>

<template>
  <el-container class="flex h-full flex-col bg-slate-50 text-slate-800">
    <!-- h-auto：解除 Element Plus 对 .el-header 的 height:60px 锁定，否则 py-3 只压缩内容区、看不出效果 -->
    <el-header class="relative flex w-full h-auto shrink-0 items-center bg-blue-500 px-6 py-3 text-white shadow-md">
      <span class="text-center text-3xl font-bold tracking-wide fn-header-title" @click="openLink(appInfo.repository)">
        {{ appInfo.name }}
      </span>
      <div>
        <el-tooltip :content="dark ? '切换亮色模式' : '切换暗色模式'" placement="bottom">
          <el-button :icon="dark ? tablerIcon(IconMoon) : tablerIcon(IconSun)" circle @click="dark = !dark"/>
        </el-tooltip>
        <el-tooltip content="使用手册" placement="bottom">
          <el-button :icon="tablerIcon(IconHelpHexagon)" circle @click="openLink(appInfo.homepage)"/>
        </el-tooltip>
        <el-tooltip content="关于" placement="bottom">
          <el-button :icon="tablerIcon(IconInfoHexagon)" circle @click="aboutVisible = true"/>
        </el-tooltip>
      </div>
    </el-header>
    <nav class="shrink-0 border-b border-slate-200 bg-white px-6 py-4">
      <el-steps :active="activeStep" finish-status="success" align-center class="fn-steps">
        <el-step
            v-for="(step, index) in steps"
            :key="step.path"
            :title="step.title"
            :description="step.description"
            :icon="tablerIcon(step.icon)"
            :class="unlocked(index) ? 'is-clickable' : 'is-locked'"
            @click="onStepClick(index)"
        />
      </el-steps>
    </nav>
    <el-main class="min-h-0 flex-1 overflow-auto p-6 fn-main">
      <router-view/>
    </el-main>
  </el-container>
  <el-dialog v-model="aboutVisible" :title="`关于 ${appInfo.name}`" width="500px">
    <div class="space-y-2 text-sm leading-6 text-slate-600">
      <p style="text-align: center;">
        <b style="font-size: 16px;">{{ appInfo.description }}</b>
      </p>
      <p>
        <b>功能</b>：通过 Excel 花名册名单，对上交的作业进行固定格式的重命名，可以自定义分隔符号，支持的 Excel
        格式：(.xlsx)(.xlsm)(.xltx)(.xltm)。
      </p>
      <p>
        <b>构建</b>：使用 <b>Tauri + Rust + Vue</b> 实现，无需安装运行环境，双击即用。
      </p>
      <p class="text-xs text-slate-400" style="display: flex;gap: 5em">
        <span><b>版本</b>：{{ appInfo.version }}</span>
        <span><b>作者</b>：{{ appInfo.authors.join(" · ") }}</span>
      </p>
      <p class="text-xs text-slate-400">
        <span><b>注意</b>：请不要把本程序和要改名的文件放在同一个文件夹里。</span>
      </p>
    </div>
  </el-dialog>
</template>

<style scoped>
/* 主内容区占满剩余高度并自行滚动，内容从顶部开始排布 */
.fn-main {
  min-height: 0;
  padding: 24px;
}

.fn-header-title {
  font-family: 'AlibabaPuHui', 'PingFang SC', 'Microsoft YaHei', system-ui, sans-serif;
  font-weight: 700;
  font-style: normal;
  line-height: 27px;
  letter-spacing: 0;
  white-space: nowrap;
}

/* 步骤条：可达的步骤给出可点击的手型与悬停反馈，未解锁的步骤禁用点击 */
.fn-steps :deep(.el-step.is-clickable) {
  cursor: pointer;
}

.fn-steps :deep(.el-step.is-clickable:hover .el-step__title.is-process),
.fn-steps :deep(.el-step.is-clickable:hover .el-step__title.is-wait),
.fn-steps :deep(.el-step.is-clickable:hover .el-step__title.is-success),
.fn-steps :deep(.el-step.is-clickable:hover .el-step__description) {
  color: var(--el-color-primary);
}

.fn-steps :deep(.el-step.is-locked) {
  cursor: not-allowed;
}

.fn-steps :deep(.el-step.is-locked .el-step__head),
.fn-steps :deep(.el-step.is-locked .el-step__main)
.fn-steps :deep(.el-step.is-locked .el-step__title),
.fn-steps :deep(.el-step.is-locked .el-step__description) {
  color: #c4c8d0;
  opacity: 0.55;
}

/* 连接线是背景色，单独调浅，避免和文字深浅不一致 */
.fn-steps :deep(.el-step.is-locked .el-step__line) {
  background-color: #e4e7ed;
  opacity: 0.55;
}

.el-step__icon svg {
  transform: scale(1.5);
  transform-origin: center;
}

.el-button svg {
  transform: scale(1.8);
  transform-origin: center;
}
</style>