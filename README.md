# Gemini App

<div align="center">

<img src="src-tauri/icons/128x128@2x.png" width="128" alt="Gemini App logo">

[![GitHub Release](https://img.shields.io/github/v/release/Benson5650/gemini-app?label=Release)](../../releases/latest)
[![GitHub Downloads](https://img.shields.io/github/downloads/Benson5650/gemini-app/total.svg?label=Downloads)](../../releases/latest)

一款輕量的 Google Gemini 桌面客戶端 透過全局快捷鍵隨時喚出 AI 助手

</div>

> 基於 [Tauri 2](https://v2.tauri.app/) 開發，支援側邊欄面板與獨立視窗兩種模式。

## Features

| 功能          | 說明                                                      |
| ------------- | --------------------------------------------------------- |
| 🖥️ 雙視窗模式 | 側邊欄面板（螢幕右側滑出）/ 正常獨立視窗                  |
| ⌨️ 全局快捷鍵 | `Ctrl+Alt+G` 側邊欄 · `Ctrl+G` 正常視窗 · `Ctrl+N` 新對話 |
| � 系統匣常駐  | 關閉時隱藏至系統匣，可自訂點擊行為                        |
| ⚙️ 豐富設定   | 面板寬度、動畫速度、自動載入對話、開機自啟動              |
| 🌐 智慧連結   | 外部連結自動在系統瀏覽器開啟                              |

## Installation

### Download

前往 [Releases](../../releases/latest) 下載最新版本：

| 格式   | 說明                     |
| ------ | ------------------------ |
| `.msi` | Windows Installer 安裝包 |
| `.exe` | NSIS 安裝程式            |

### Build from source

1. 安裝 [Node.js LTS](https://nodejs.org/) 及
   [Rust](https://www.rust-lang.org/tools/install)
2. `npm install` 安裝依賴
3. `npm run tauri dev` 開發模式
4. `npm run tauri build` 建置正式版

> 建置產物位於 `src-tauri/target/release/bundle/`

## Release

推送版本 Tag 即可自動建置與發布：

```bash
git tag v0.1.0
git push origin v0.1.0
```

<div align="center">

Built with ❤️ using [Tauri 2](https://v2.tauri.app/) +
[Rust](https://www.rust-lang.org/)

</div>
