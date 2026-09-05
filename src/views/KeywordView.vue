<script lang="ts" setup>
import {computed} from 'vue'
import {useRouter} from 'vue-router'
import {ElMessage} from 'element-plus'
import {ArrowLeft, Key, Select} from '@element-plus/icons-vue'
import {submitData} from '@/api'
import {columns} from '@/store'

const router = useRouter()

/** 把列数据转换成 el-table 需要的行数据 */
const rows = computed(() => {
  const size = columns.value[0]?.values.length ?? 0
  return Array.from({length: size}, (_, i) => {
    const row: Record<string, string> = {}
    columns.value.forEach((col, j) => {
      row[String(j)] = col.values[i] ?? ''
    })
    return row
  })
})

const keywordCount = computed(() => columns.value.filter((c) => c.isKeyWord).length)

function headerClassName({column}: { column: { property?: string } }) {
  const index = Number(column?.property)
  const col = columns.value[index]
  if (!col?.isKeyWord) return ''
  return 'keyword-header'
}

function toggleHeader(column: { property?: string }) {
  const index = Number(column?.property)
  const col = columns.value[index]
  if (!col) return
  col.isKeyWord = !col.isKeyWord
}

async function submit() {
  if (keywordCount.value === 0) {
    ElMessage.error('请至少选择一个关键字')
    return
  }
  try {
    const res = await submitData(columns.value)
    if (res.code === 0) {
      ElMessage.success(res.msg)
      router.push('/format')
    } else {
      ElMessage.error(res.msg)
    }
  } catch (e) {
    ElMessage.error(`提交失败：${e}`)
  }
}
</script>

<template>
  <div class="mx-auto flex h-full w-[90vw] flex-col">
    <el-card body-class="!p-4" class="!rounded-2xl" shadow="never">
      <template #header>
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-2 text-lg font-semibold text-brand">
            <el-icon>
              <Key/>
            </el-icon>
            <span>第二步：选择关键字</span>
          </div>
          <el-tag effect="dark" round type="primary">
            已选 {{ keywordCount }} / {{ columns.length }}
          </el-tag>
        </div>
      </template>

      <el-alert
          :closable="false"
          class="mb-4"
          show-icon
          title="点击表头可以选择关键字，鼠标悬停在表头上可查看不能作为关键字的原因"
          type="info"
      />

      <el-table
          :data="rows"
          :header-cell-class-name="headerClassName"
          border
          max-height="460"
          stripe
          @header-click="toggleHeader"
      >
        <el-table-column
            v-for="(col, index) in columns"
            :key="index"
            :label="col.key"
            :prop="String(index)"
            min-width="140"
        >
          <template #header>
            <el-tooltip :content="col.reason" effect="dark" placement="top">
              <span class="inline-flex items-center gap-1">
                <span>{{ col.key }}</span>
                <el-tag v-if="!col.display" effect="dark" size="small" type="danger">
                  非法字符
                </el-tag>
              </span>
            </el-tooltip>
          </template>
          <template #default="{ row }">
            <span class="whitespace-pre">{{ row[String(index)] }}</span>
          </template>
        </el-table-column>
      </el-table>

      <div class="mt-5 flex items-center gap-3">
        <el-button :icon="ArrowLeft" size="large" @click="router.push('/excel')">返回</el-button>
        <el-button :icon="Select" size="large" type="primary" @click="submit">提交</el-button>
      </div>
    </el-card>
  </div>
</template>
