import {createApp} from "vue";
import AppVue from "./App.vue";

import ElementPlus from 'element-plus'
import zhCn from 'element-plus/es/locale/lang/zh-cn'
// Tailwind(含 preflight)需先于 Element Plus 引入, 避免重置样式覆盖组件库
import './style.css'
import 'element-plus/dist/index.css'
// Element Plus 暗色变量，作用于 html.dark
import 'element-plus/theme-chalk/dark/css-vars.css'
import router from './router'

createApp(AppVue)
    .use(router)
    .use(ElementPlus, {locale: zhCn})
    .mount('#app')
