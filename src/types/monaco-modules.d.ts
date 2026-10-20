// monaco 的导入全部走 vite alias 直链包内真实文件（见 vite.config.ts）：
// rolldown-vite 对 monaco 包子路径解析不可靠（`?worker` 查询导入不转换、
// worker 子构建解析失败，实测 8.3.3）。alias 模块的类型在此挂到包根声明。

declare module "monaco-api" {
  export * from "monaco-editor";
}

declare module "monaco-json-contribution" {
  // 纯 side-effect 模块：注册 JSON 语言与 worker 客户端，无导出
}

declare module "monaco-json-worker" {
  // 纯 side-effect 模块：worker 入口
}

declare module "monaco-editor-worker" {
  // 纯 side-effect 模块：worker 入口
}
