import {invoke} from '@tauri-apps/api/core'
import type {ApiResp, AppInfoResp, ColData, ExecuteItem, ExecuteResp} from '@/types'

/** 获取 APP 信息, 包含用用名称、版本号、描述、作者信息 */
export const getAppInfo = () => invoke<AppInfoResp>('get_app_info')

/** 获取当前程序的数据（Excel 解析后的列数据） */
export const getData = () => invoke<ColData[]>('get_data')

/** 提交 excel 的路径 */
export const submitExcelPath = (path: string) =>
    invoke<ApiResp>('submit_excel_path', {path})

/** 提交表格文本 */
export const submitTableText = (text: string) =>
    invoke<ApiResp>('submit_table_text', {text})

/** 提交关键字配置 */
export const submitData = (data: ColData[]) => invoke<ApiResp>('submit_data', {data})

/** 判断路径是否是一个文件夹（拖拽选择文件夹时校验用） */
export const isDir = (path: string) => invoke<boolean>('is_dir', { path })

/** 提交要改名的文件夹路径与命名格式 */
export const submitExecute = (path: string, execute: ExecuteItem[]) =>
    invoke<ApiResp>('submit_execute', {path, execute})

/** 获取本次改名的操作以及新旧名字 */
export const getExecute = () => invoke<ExecuteResp>('get_execute')

/** 重新扫描目录并重新计算新旧名字对照 */
export const rescan = () => invoke<ApiResp>('rescan')

/** 发起重命名 */
export const rename = () => invoke<ApiResp>('rename')

/** 恢复原文件名 */
export const recover = () => invoke<ApiResp>('recover')

/** 备份当前目录中的文件 */
export const backup = () => invoke<ApiResp>('backup')

/** 清空已导入的数据 */
export const clearData = () => invoke<ApiResp>('clear_data')
