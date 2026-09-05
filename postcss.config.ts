import autoprefixer from 'autoprefixer'
import tailwindcss from 'tailwindcss'

/**
 * PostCSS 配置: 让 Vite 在构建时处理 @tailwind 指令
 */
export default {
  plugins: {
    tailwindcss: {},
    autoprefixer: {},
  },
}
