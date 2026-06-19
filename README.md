# 小剪

小剪是一款本地优先的 Windows 剪贴板增强工具。它会自动保存复制过的文本和图片，支持搜索、收藏、置顶、快捷键绑定、候选面板快速粘贴，以及可选的 DeepSeek 中英互译。项目基于 Tauri 2 构建，前端使用原生 HTML/CSS/JavaScript，后端使用 Rust，安装包体积轻，数据默认只保存在本机。

## 项目简介

很多临时复制内容会在下一次复制时被覆盖，小剪的目标是把这些短暂内容变成可检索、可复用、可快速粘贴的本地资料库。它适合写作、客服、开发、运营、资料整理等需要频繁复制粘贴的场景。

核心特点：

- 本地保存剪贴板历史，不需要账号登录或云同步。
- 自动记录文本和图片，图片以本地 PNG 保存并生成缩略图。
- 支持收藏和置顶，重要记录不会被普通历史清理影响。
- 支持 `Ctrl + F1` 到 `Ctrl + F11` 绑定常用记录，一键复制。
- 支持全局候选面板，默认快捷键 `Ctrl + Shift + V`，可搜索并快速粘贴。
- 支持系统托盘、开机自启动、最小化到托盘。
- 内置曜石 / 晶透双主题皮肤，一键切换并实时同步主窗口与候选面板。
- 可选接入 DeepSeek API，对文本记录做中英互译。
- 支持生成 Windows NSIS 安装包，可选择安装路径并创建桌面快捷方式。

## 功能清单

| 功能 | 说明 |
| --- | --- |
| 剪贴板监听 | 监听系统剪贴板变化，自动保存文本和图片 |
| 历史记录 | 查看、搜索、复制、删除历史记录 |
| 收藏记录 | 收藏重要内容，支持置顶和收藏内搜索 |
| 快捷绑定 | 将记录绑定到 `Ctrl + F1` 到 `Ctrl + F11` |
| 候选面板 | 全局快捷键唤起，支持最近记录、收藏记录和搜索结果 |
| 自动粘贴 | 选中候选项后写入剪贴板并尝试模拟 `Ctrl + V` |
| 设置管理 | 修改唤起快捷键、历史容量、开机自启、托盘行为 |
| 数据清理 | 清空普通历史或清空全部收藏，带二次确认 |
| 图片记录 | 保存剪贴板图片，展示缩略图，支持复制、收藏和绑定 |
| 皮肤主题 | 曜石（深色实体）与晶透（毛玻璃）双皮肤，切换即时生效并跨窗口同步 |
| AI 翻译 | 配置 DeepSeek API Key 后对文本记录进行中英互译 |
| 安装包 | 使用 Tauri 生成 Windows NSIS 安装包 |

## 安装使用

### 下载安装包

在本仓库的 Releases 页面下载 `xiaojian_1.1.0_x64-setup.exe` 后运行安装。

安装器支持：

- 选择安装路径
- 创建开始菜单快捷方式
- 安装完成后创建桌面快捷方式
- 自动检测并下载 WebView2 运行时

> WebView2 是 Tauri Windows 应用的运行依赖。Windows 11 通常已内置，部分 Windows 10 设备首次安装时需要联网下载。

### 从源码运行

环境要求：

- Windows 10 或更高版本
- Node.js 18+
- Rust 1.77+
- WebView2 Runtime

安装依赖：

```powershell
npm install
```

开发运行：

```powershell
npm run dev
```

构建安装包：

```powershell
npm run build
```

构建成功后，安装包位于：

```text
src-tauri/target/release/bundle/nsis/
```

为了方便分发，也可以将安装包复制到项目根目录的 `dist/` 目录。`dist/` 默认不提交到 git。

## 使用说明

### 历史记录

打开小剪后，在任意软件中复制文本或图片，小剪会自动保存到历史记录。历史页支持按关键词搜索，也可以按全部、文本、图片筛选。

鼠标悬停单条记录可进行：

- 复制
- 收藏
- 绑定快捷键
- 删除

### 收藏和置顶

收藏记录会进入收藏页。收藏记录不计入普通历史容量上限，也不会被“清空普通历史”删除。收藏后可以继续置顶，置顶记录会显示在收藏页上方。

### 快捷绑定

可以把任意记录绑定到 `Ctrl + F1` 到 `Ctrl + F11`。绑定后，在任意软件按对应快捷键，就会把绑定记录写入系统剪贴板。

### 候选面板

默认按 `Ctrl + Shift + V` 唤起候选面板。面板会显示最近记录和收藏记录，也可以输入关键词搜索。

快捷操作：

- `↑` / `↓`：移动选择
- `Enter`：确认并粘贴
- `Esc`：关闭面板

### 皮肤主题

在侧边栏「皮肤」页可以切换整体外观，目前内置两套：

- **曜石**：深色实体面板，原生桌面质感（默认）。
- **晶透**：半透明毛玻璃，科技氛围光感。

选择后立即生效，主窗口与候选面板会同步切换，并记住你的选择。

### DeepSeek 翻译

在设置页开启 DeepSeek 翻译并填写 API Key 后，文本记录会出现“翻译”操作。小剪会自动判断中文转英文或英文转中文。

说明：

- API Key 保存在本机，并使用 Windows DPAPI 加密。
- 只有点击“翻译”时才会联网请求 DeepSeek。
- 未启用翻译时，其余剪贴板功能均为本地功能。

## 数据存储

小剪的运行数据不在安装包里，也不会提交到 GitHub。Windows 下默认保存在当前用户的应用数据目录：

```text
%APPDATA%\com.wanghan.clipboardenhancer\
```

主要文件：

```text
clipboard-history.json      剪贴板历史记录
settings.json               应用设置
hotkey-bindings.json        Ctrl+F1~F11 绑定关系
ai-config.json              DeepSeek 配置
images\                     剪贴板图片原图
```

如果需要在本机模拟首次安装，可以退出小剪后删除上述目录。

WebView2 缓存和日志位于：

```text
%LOCALAPPDATA%\com.wanghan.clipboardenhancer\
```

## 项目结构

```text
小剪/
├─ ui/
│  ├─ main.html          主窗口页面
│  ├─ main.css           主窗口样式
│  ├─ main.js            主窗口交互逻辑
│  ├─ panel.html         候选面板页面
│  ├─ panel.css          候选面板样式
│  ├─ panel.js           候选面板交互逻辑
│  ├─ shim.js            Tauri API 兼容桥接
│  ├─ theme.js           皮肤（主题）系统，首屏防闪烁切换
│  └─ planet.js          监听状态粒子动画
├─ src-tauri/
│  ├─ src/
│  │  ├─ ai.rs           DeepSeek 翻译
│  │  ├─ clipboard.rs    剪贴板监听和写入
│  │  ├─ commands.rs     前端 IPC 命令
│  │  ├─ hotkeys.rs      全局快捷键
│  │  ├─ lib.rs          Tauri 应用入口和插件注册
│  │  ├─ paste.rs        自动粘贴
│  │  ├─ secret.rs       DPAPI 加密
│  │  ├─ state.rs        应用状态
│  │  ├─ store.rs        本地数据存储
│  │  ├─ tray.rs         系统托盘
│  │  └─ window.rs       窗口管理
│  ├─ capabilities/      Tauri 权限配置
│  ├─ icons/             应用图标
│  ├─ nsis-hooks.nsh     NSIS 安装器 hook
│  ├─ tauri.conf.json
│  └─ tauri.windows.conf.json
├─ package.json
├─ package-lock.json
└─ README.md
```

## 技术栈

- Tauri 2
- Rust 2021
- HTML / CSS / JavaScript
- tauri-plugin-clipboard
- tauri-plugin-global-shortcut
- tauri-plugin-autostart
- tauri-plugin-single-instance
- tauri-plugin-log
- reqwest
- windows-dpapi
- enigo
- image

## 打包说明

项目已配置 Windows NSIS 安装包：

- `src-tauri/tauri.conf.json`：基础 Tauri 配置
- `src-tauri/tauri.windows.conf.json`：Windows 安装器补充配置
- `src-tauri/nsis-hooks.nsh`：安装完成后创建桌面快捷方式

构建命令：

```powershell
npm run build -- --bundles nsis
```

## 隐私说明

小剪默认只在本机保存数据，不需要账号，也不会上传剪贴板历史。

唯一会主动联网的功能是 DeepSeek 翻译。该功能需要用户手动开启，并且只会在点击“翻译”某条文本记录时发送该文本内容。

## 许可证

MIT License
