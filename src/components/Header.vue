<template>
  <header :class="{'is-dark': dark}" class="fn-header">
    <div class="fn-header__title">{{ title }}</div>

    <div class="fn-header__actions">
      <button
          v-if="showThemeToggle"
          :aria-label="themeLabel"
          :title="themeLabel"
          class="fn-icon-btn"
          type="button"
          @click="toggleTheme"
      >
        <span class="fn-icon">
          <!-- 太阳：浅色模式 -->
          <svg v-if="dark" aria-hidden="true" viewBox="0 0 32 32">
            <g fill="currentColor">
              <circle cx="16" cy="16" r="6"/>
              <rect height="4.5" rx="1" width="2" x="15" y="2"/>
              <rect height="4.5" rx="1" width="2" x="15" y="25.5"/>
              <rect height="2" rx="1" width="4.5" x="2" y="15"/>
              <rect height="2" rx="1" width="4.5" x="25.5" y="15"/>
              <rect height="4.5" rx="1" transform="rotate(45 16 16)" width="2" x="15" y="2"/>
              <rect height="4.5" rx="1" transform="rotate(-45 16 16)" width="2" x="15" y="2"/>
              <rect height="4.5" rx="1" transform="rotate(135 16 16)" width="2" x="15" y="2"/>
              <rect height="4.5" rx="1" transform="rotate(-135 16 16)" width="2" x="15" y="2"/>
            </g>
          </svg>
          <!-- 月亮：深色模式 -->
          <svg v-else aria-hidden="true" viewBox="0 0 32 32">
            <path
                d="M10.688 8.224c0-1.984 0.448-3.84 1.28-5.536 0.256-0.544 0.032-0.832-0.544-0.608-5.504 2.048-9.408 7.264-9.408 13.472 0 7.968 6.464 14.432 14.432 14.432 6.208 0 11.456-3.904 13.472-9.408 0.224-0.576-0.064-0.8-0.608-0.512-1.618 0.788-3.521 1.248-5.531 1.248-0.002 0-0.004 0-0.006 0h0c-7.232 0-13.088-5.856-13.088-13.088z"
                fill="currentColor"
            />
          </svg>
        </span>
      </button>

      <button
          aria-label="使用手册"
          class="fn-icon-btn"
          title="使用手册"
          type="button"
          @click="emit('help')"
      >
        <span class="fn-icon">
          <svg aria-hidden="true" viewBox="64 64 896 896">
            <g fill="currentColor">
              <path
                  d="M512 64C264.6 64 64 264.6 64 512s200.6 448 448 448 448-200.6 448-448S759.4 64 512 64zm0 820c-205.4 0-372-166.6-372-372s166.6-372 372-372 372 166.6 372 372-166.6 372-372 372z"/>
              <path
                  d="M623.6 316.7C593.6 290.4 554 276 512 276s-81.6 14.5-111.6 40.7C369.2 344 352 380.7 352 420v7.6c0 4.4 3.6 8 8 8h48c4.4 0 8-3.6 8-8V420c0-44.1 43.1-80 96-80s96 35.9 96 80c0 31.1-22 59.6-56.1 72.7-21.2 8.1-39.2 22.3-52.1 40.9-13.1 19-19.9 41.8-19.9 64.9V620c0 4.4 3.6 8 8 8h48c4.4 0 8-3.6 8-8v-22.7a48.3 48.3 0 0 1 30.9-44.8c59-22.7 97.1-74.7 97.1-132.5.1-39.3-17.1-76-48.3-103.3zM472 732a40 40 0 1 0 80 0 40 40 0 1 0-80 0z"/>
            </g>
          </svg>
        </span>
      </button>

      <button
          aria-label="关于"
          class="fn-icon-btn"
          title="关于"
          type="button"
          @click="emit('about')"
      >
        <span class="fn-icon">
          <svg aria-hidden="true" viewBox="64 64 896 896">
            <g fill="currentColor">
              <path
                  d="M512 64C264.6 64 64 264.6 64 512s200.6 448 448 448 448-200.6 448-448S759.4 64 512 64zm0 820c-205.4 0-372-166.6-372-372s166.6-372 372-372 372 166.6 372 372-166.6 372-372 372z"/>
              <path
                  d="M464 336a48 48 0 1 0 96 0 48 48 0 1 0-96 0zm72 112h-48c-4.4 0-8 3.6-8 8v272c0 4.4 3.6 8 8 8h48c4.4 0 8-3.6 8-8V456c0-4.4-3.6-8-8-8z"/>
            </g>
          </svg>
        </span>
      </button>
    </div>
  </header>
</template>

<script lang="ts" setup>
import {computed} from 'vue'

const props = withDefaults(
    defineProps<{
      /** 标题文字 */
      title?: string
      /** 是否为深色模式，支持 v-model:dark */
      dark?: boolean
      /** 是否显示主题切换按钮 */
      showThemeToggle?: boolean
    }>(),
    {
      title: 'Format Name',
      dark: false,
      showThemeToggle: true,
    }
)

const emit = defineEmits<{
  'update:dark': [value: boolean]
  'theme-toggle': [value: boolean]
  help: []
  about: []
}>()

const themeLabel = computed(() => (props.dark ? '切换为浅色模式' : '切换为深色模式'))

function toggleTheme() {
  const next = !props.dark
  emit('update:dark', next)
  emit('theme-toggle', next)
}
</script>

<style scoped>
.fn-header {
  box-sizing: border-box;
  display: flex;
  width: 100%;
  height: 50px;
  align-items: center;
  justify-content: space-between;
  padding: 0 40px;
  border-bottom: 1px solid rgba(0, 18, 38, 0.06);
  background: rgba(251, 251, 251, 1);
  user-select: none;
  -webkit-user-select: none;
  transition: background-color 0.2s ease, border-color 0.2s ease;
}

.fn-header__title {
  color: rgb(16, 16, 16);
  font-family: 'AlibabaPuHui', 'PingFang SC', 'Microsoft YaHei', system-ui, sans-serif;
  font-size: 20px;
  font-weight: 700;
  font-style: normal;
  line-height: 27px;
  letter-spacing: 0;
  white-space: nowrap;
}

.fn-header__actions {
  display: flex;
  align-items: center;
  gap: 24px;
}

.fn-icon-btn {
  display: flex;
  width: 40px;
  height: 40px;
  align-items: center;
  justify-content: center;
  padding: 0;
  margin: 0;
  border: 0;
  border-radius: 100%;
  background: rgba(240, 240, 240, 1);
  color: rgba(134, 144, 156, 1);
  appearance: none;
  cursor: pointer;
  transition: background-color 0.18s ease, color 0.18s ease;
}

.fn-icon-btn:hover {
  background: rgba(229, 230, 235, 1);
  color: rgb(78, 89, 105);
}

.fn-icon-btn:active {
  background: rgba(220, 220, 220, 1);
}

.fn-icon-btn:focus-visible {
  outline: 2px solid #602f88;
  outline-offset: 2px;
}

.fn-icon {
  display: block;
  width: 20px;
  height: 20px;
}

.fn-icon svg {
  display: block;
  width: 100%;
  height: 100%;
  vertical-align: top;
}

/* 深色模式 */
.fn-header.is-dark {
  border-bottom-color: rgba(255, 255, 255, 0.08);
  background: rgb(23, 23, 26);
}

.fn-header.is-dark .fn-header__title {
  color: rgb(246, 246, 246);
}

.fn-header.is-dark .fn-icon-btn {
  background: rgb(42, 42, 46);
  color: rgb(169, 174, 184);
}

.fn-header.is-dark .fn-icon-btn:hover {
  background: rgb(53, 53, 59);
  color: rgb(229, 230, 235);
}

.fn-header.is-dark .fn-icon-btn:active {
  background: rgb(64, 64, 71);
}

.fn-header.is-dark .fn-icon-btn:focus-visible {
  outline-color: #a97fc0;
}
</style>
