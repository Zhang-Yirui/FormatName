import type { Config } from 'tailwindcss'

/**
 * Tailwind CSS 配置
 */
export default {
  // 扫描模板中使用了类名的文件
  content: ['./index.html', './src/**/*.{vue,js,ts,jsx,tsx}'],
  important: true,
  // 由 <html class="dark"> 手动控制暗色模式（不跟随系统自动切换）
  darkMode: 'class',
  theme: {
    extend: {
      colors: {
        // 与 Element Plus 主题色 --el-color-primary 保持一致
        brand: {
          DEFAULT: '#409eff',
          light: '#66b1ff',
          dark: '#337ecc',
        },
      },
    },
  },
  plugins: [],
} satisfies Config
