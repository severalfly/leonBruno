# leonBruno

一个**本地优先、Git 友好、低内存**的 API 客户端，用 Tauri 2 + Vue 3 + Rust 写的 Windows 便携应用。名字致敬 Bruno，但针对它的几个痛点做了重新设计。

## 为什么做这个

Bruno 很好用——离线、集合以纯文本 `.bru` 存本地、天然的 Git 工作流。但用下来有几个问题：

| Bruno 的问题 | leonBruno 的做法 |
|---|---|
| 不能把 cURL 直接粘贴进来，参数识别不出来 | 顶部「粘贴 cURL」，自动解析 method / URL / Header / Query / Body / Bearer / Basic Auth，支持 DevTools 的 Copy as cURL |
| 按项目管理标签页，频繁切项目会切换标签页分组 | 标签页**全局扁平**，不属于任何集合；切集合、切环境都不动标签 |
| 内存占用 200MB（Electron） | Tauri + WebView2，常驻约 60–90MB；请求用 Rust reqwest 异步发送，前端不留大对象 |
| 集合格式 | **直接继承 Bruno 的两种原生格式**：`.bru`（Bru Markup Language）与 OpenCollection YAML（`.yml`）。写出来的集合 Bruno 能直接打开，Bruno 现有集合也能直接读入 |

## 功能（v0.1.0）

- 请求编辑：GET/POST/PUT/PATCH/DELETE/HEAD/OPTIONS，URL、Query、Headers、Body、Bearer/Basic Auth
- **cURL 粘贴导入**：`curl -X POST 'https://api...' -H 'Authorization: Bearer xxx' -d 'a=b'` 一键变成可编辑请求
- 全局扁平标签页；左侧集合树、中间请求编辑、右侧响应
- 本地集合：Bruno 原生 `.bru` / OpenCollection `.yml`，集合目录含 `opencollection.yml`（新格式）或 `bruno.json` + `collection.bru`（传统格式）+ `environments/*.bru`，与 Bruno 双向兼容，可直接放进 Git 仓库
- `{{变量名}}` 插值 + 多环境切换，env 文件存在本地
- 响应查看：状态码、耗时、大小、Body、Headers
- **导入现有 Bruno 仓库**：侧边栏「导入仓库」填本机目录，直接读取原目录（不复制、不改你的 Git 仓库），Bruno 与 leonBruno 可共用同一份集合；支持 OpenCollection YAML（`opencollection.yml`）、传统 `.bru`、混合布局，可识别仓库根下的多个集合子目录
- 便携模式：exe 同目录 `portable.dat` 即把数据写到 `data/`，不写注册表

## 技术栈

- 前端：Vue 3 + TypeScript + Vite
- 后端：Rust / Tauri 2 / reqwest（rustls，无 OpenSSL 依赖）
- 构建目标：`x86_64-pc-windows-gnu`（MSYS2 GCC）

## 开发

```bash
npm install
npm run tauri dev
# 构建
npm run build
npm run tauri:build
npm run portable
```

数据目录：便携模式下是 exe 同目录 `data/`（集合在 `data/workspace/`，设置在 `data/settings.json`）。集合格式在「设置 → 集合格式」里选，默认 `.yml`（OpenCollection YAML，Bruno 新推荐），也支持 `.bru`（Bru 原生）；0.1.0 的旧 `.json` 请求会在打开集合时自动转成 `.bru`。

**内存与生命周期**：leonBruno 不是常驻托盘程序，关掉窗口进程就退出、内存归零。实测运行期 Working Set 约 **22MB**（Windows VM，`tasklist` 读数），不存在「常驻 60MB」。

## License

MIT
