/**
 * 表格中的一列数据 (对应 Rust 侧的 ColData)
 */
export interface ColData {
    /** 表头 */
    key: string
    /** 该列除表头以外的所有值 */
    values: string[]
    /** 是否作为关键字 */
    isKeyWord: boolean
    /** 是否允许出现在命名格式中 */
    display: boolean
    /** 系统给出的原因说明 */
    reason: string
    /** 该列值的重复次数, 用于给关键字排序 (越小优先级越高) */
    delta: number
}

/** 命名格式中的一项：列序号 / 空占位 / 自定义文本 */
export type ExecuteItem = number | null | string

/** 命名段类型：字段 / 分隔符 / 自定义文本 */
export type SegType = 'keyword' | 'connector' | 'custom'

/** 命名构建区中的一个命名段 */
export interface RuleSegment {
    /** 仅用于列表渲染与拖拽，不参与改名 */
    id: string
    type: SegType
    /** keyword 为列序号，connector / custom 为文本 */
    value: number | string
}

/** 命名规则：构建区内容 + 分隔符设置，跨页面保留，回到命名格式页时原样回填 */
export interface FormatRule {
    segments: RuleSegment[]
    /** 是否统一分隔符 */
    same: boolean
    /** 统一分隔符选中的选项值 */
    sep: string
    /** 自定义分隔符内容 */
    customSep: string
    /** 已添加过的自定义连接符 */
    customConnectors: string[]
}

/** 统一接口返回结构 */
export interface ApiResp {
    code: number
    msg: string
}

/** 新旧名字对照 */
export interface NamePair {
    old: string
    new: string
}

/** 一次改名任务的执行结果 */
export interface ExecuteResp {
    /** 每个名字被匹配到的次数标记 */
    map: number[]
    /** 新旧名字对照表 */
    list: NamePair[]
    /** 新名字列表 */
    new: string[]
    /** 目录中的旧文件名 */
    old: string[]
    /** 是否已改名 */
    flag: number
}

export interface AppInfoResp {
    name: string,
    version: string,
    description: string,
    authors: string[],
    license: string,
    repository: string,
    homepage: string,
}
