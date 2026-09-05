<script setup lang="ts">
import {computed, onMounted, ref} from 'vue'
import {useRoute} from 'vue-router'
import {openUrl} from '@tauri-apps/plugin-opener'
import {getAppInfo} from '@/api'
import {
  IconHelpHexagon, IconHexagonNumber1, IconHexagonNumber2, IconHexagonNumber3, IconHexagonNumber4, IconInfoHexagon,
  IconMoon,
  IconSun,
} from '@tabler/icons-vue';
import {AppInfoResp} from "@/types";
import {tablerIcon} from '@/utils/tablerIcon'

const route = useRoute()
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
    <el-header class="relative flex w-full shrink-0 items-center bg-blue-500 px-6 py-3 text-neutral-900 shadow-md">
      <span class="text-center text-3xl font-bold tracking-wide fn-header__title" @click="openLink(appInfo.repository)">
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
      <el-steps :active="activeStep" finish-status="success" align-center>
        <el-step title="选择 Excel" description="导入花名册" :icon="tablerIcon(IconHexagonNumber1, )"/>
        <el-step title="选择关键字" description="点击表头切换" :icon="tablerIcon(IconHexagonNumber2, )"/>
        <el-step title="设置命名格式" description="自定义分隔符" :icon="tablerIcon(IconHexagonNumber3, )"/>
        <el-step title="数据分析" description="改名 / 恢复 / 备份" :icon="tablerIcon(IconHexagonNumber4, )"/>
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
.fn-main {
  min-height: 0;
  padding: 24px;
}

.fn-header__title {
  font-family: 'AlibabaPuHui', 'PingFang SC', 'Microsoft YaHei', system-ui, sans-serif;
  font-weight: 700;
  font-style: normal;
  line-height: 27px;
  letter-spacing: 0;
  white-space: nowrap;
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