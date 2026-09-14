export const STEPS = [
    { title: '导入表格', desc: '导入花名册', path: '/excel', hint: '选择或拖入花名册，支持 Excel、CSV 表格。' },
    { title: '选择关键字', desc: '点击表头选择', path: '/keyword', hint: '点击表头选择关键字，关键字用于把文件名和花名册中的记录对应起来，蓝色表头即为已选。' },
    { title: '设置命名格式', desc: '可自定义分隔符', path: '/format', hint: '组合字段与分隔符，并选择待改名的文件夹。' },
    { title: '数据分析', desc: '改名 / 恢复 / 备份', path: '/analysis', hint: '核对新旧名字对照，确认无误后再执行改名。' },
]

/** Excel 走后端按路径读取 */
export const EXCEL_EXT = ['xlsx', 'xlsm', 'xltx', 'xltm']

/** 文本表格在后端按 CSV 解析 */
export const TEXT_EXT = ['csv', 'txt']
