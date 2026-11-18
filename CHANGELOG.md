# Changelog

## [0.2.2] - 2026-11-18

### 修复

- **导入 OpenCollection 新格式仓库失败**：用户的 Bruno 仓库用新格式，每个集合目录里是 `opencollection.yml`（含 `info.name`）而不是旧的 `bruno.json` / `collection.bru`，导致「导入仓库」识别失败。现在 `probe_bruno_repo` / `scan_root` 都认 `opencollection.yml` / `opencollection.yaml`，集合名从 `info.name` 读取，并把 `opencollection.yml` 自身排除出请求列表（它是集合定义，不是请求）。仓库根下的多个集合子目录全部识别。
- **递归读取子目录分组的请求**：Bruno 用子目录给请求分组（如 `act11/account/folder.yml`、`act11/act/...`），原来只读集合第一层会漏掉绝大多数请求。现在递归读取所有层级的 `.yml`/`.bru`（跳过 `environments`/`.git`/`node_modules` 与 `folder.yml` 定义文件），请求 id 用相对路径前缀保证跨子目录重名不冲突。实测用户真实仓库 `D://work//docs//bruno`：识别 28 个集合、658 个请求（此前只读到 26 个），其中 19 个重名文件靠路径区分。
- **URL scheme 智能补全**：0.2.1 为修 `baidu.com` 把裸域名统一补 `https://`，但 Bruno 集合里常见的 `localhost:8080/...`、`192.168.x.x`、`10.x`、`172.x`、`*.local` 会被错误补成 https 导致本地服务请求失败。现在本地/内网地址补 `http://`，公网补 `https://`。

### 验证

- 新增单测 `opencollection_collection_name_and_skip`：验证 `opencollection.yml` 的 `info.name` 被识别为集合名、该文件被跳过、同目录 `.yml` 请求被正确解析。
- vue-tsc + vite + Tauri 构建通过。

## [0.2.1] - 2026-11-18

### 修复与改进

- **修复「baidu.com 发送失败」**：URL 没写协议时原来补 `http://`，`http://baidu.com` 会 301 跳到 https，跨协议重定向时 reqwest 会丢掉 headers 并可能失败。现在补 `https://`。用真实网络测试验证（VM 上直接发 `baidu.com` 与 `https://www.baidu.com` 均返回 200）。
- **错误可读**：展开 reqwest 的 source 错误链，失败时能看到底层真正原因（DNS / TLS / 连接 / 超时）而不只是一句 `error sending request`。
- **默认格式改为 YAML**：新装用户默认用 OpenCollection YAML（`.yml`，Bruno 现在推荐的新格式）；仍可切换回 `.bru`。
- **导入现有 Bruno 仓库**：侧边栏「导入仓库」填本机目录路径，先识别再导入。leonBruno **直接读取原目录**，不复制、不改动 Git 仓库；支持单个集合目录，也支持包含多个集合的仓库根目录。集合自带的 `environments/*.bru` 会一并读入。已导入的仓库可在弹窗里移除。

### 验证

- Rust 单测 14/14：12 个解析测试 + 2 个真实网络测试（`baidu.com` 域名直输、https 直连）。
- 前端类型检查与构建通过。

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
