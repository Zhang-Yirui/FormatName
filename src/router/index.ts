import { createRouter, createWebHashHistory, RouteLocationNormalized } from 'vue-router'
import { loadColumns, reachStep } from '@/store'
import ExcelView from '@/views/ExcelView.vue'
import KeywordView from '@/views/KeywordView.vue'
import FormatView from '@/views/FormatView.vue'
import AnalysisView from '@/views/AnalysisView.vue'

const router = createRouter({
    history: createWebHashHistory(),
    routes: [
        { path: '/', redirect: '/excel' },
        { path: '/excel', name: 'excel', component: ExcelView, meta: { step: 0 } },
        { path: '/keyword', name: 'keyword', component: KeywordView, meta: { step: 1 } },
        { path: '/format', name: 'format', component: FormatView, meta: { step: 2 } },
        { path: '/analysis', name: 'analysis', component: AnalysisView, meta: { step: 3 } },
    ],
})

router.beforeEach(async (to: RouteLocationNormalized) => {
    // 进入 Excel 页之外的页面前，确保数据已经加载
    if (to.path !== '/excel') {
        const data = await loadColumns()
        if (data.length === 0) {
            return { path: '/excel' }
        }
    }
    return true
})

// 记录到达过的最远步骤，步骤条据此判断哪些步骤可以点击跳转
router.afterEach((to: RouteLocationNormalized) => {
    reachStep(Number(to.meta.step ?? 0))
})

export default router
