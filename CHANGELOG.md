# Changelog

## [0.2.0] - 2026-11-18

### 集合存储改为 Bruno 原生格式（双向兼容）

- 新增 `.bru` 解析 / 序列化器：支持 `meta`（name/type/seq）、`get/post/put/patch/delete/head/options` 方法块、`params:query`、`headers`、`body/json/text/xml/form-urlencoded/graphql`、`auth:bearer/basic/apikey`；`~` 前缀识别为禁用行；`@description` 忽略。
- 新增 OpenCollection YAML 读写（`info/http/runtime/settings`），Bruno 当前推荐的新格式。设置里可切换保存格式，默认 `.bru`。
- 集合目录按 Bruno 布局：`bruno.json`（集合名）+ `collection.bru` + `*.bru|*.yml` 请求 + `environments/*.bru`（`vars{}` / `vars:secret`）。Bruno 可直接打开 leonBruno 写的集合，leonBruno 也能直接读 Bruno 现有集合。
- 0.1.0 的旧 `.json` 请求在打开集合时自动转成 `.bru` 并移除原文件。
- 请求新增 `seq` 字段，按 Bruno 的序号排序。

### 验证

- Rust 单元测试 12/12：Bru 官方样例、带 body/auth/禁用参数解析、序列化往返、YAML 样例、Bruno 目录解析、5 个 curl 解析测试。
- 实测运行期内存：Working Set **22.3MB**，关闭窗口后无残留进程 —— 不存在常驻占用。

## [0.1.0] - 2026-11-18

首个版本。

- 新增：Tauri 2 + Vue 3 + Rust 的 Windows 便携应用骨架
- 新增：cURL 命令解析粘贴导入（method/URL/Header/Query/Body/Bearer/Basic/User-Agent/Cookie）
- 新增：全局扁平标签页，切换集合与环境不重组标签
- 新增：本地纯文本 JSON 集合存储（一个集合一个目录、一个请求一个文件），可直接 Git 管理
- 新增：`{{变量名}}` 插值与多环境切换
- 新增：HTTP 请求发送与响应查看（status/time/size/body/headers）
- 新增：便携模式（portable.dat）
