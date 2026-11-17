<p align="right"><b>繁體中文</b> | <a href="README.en.md">English</a></p>

<div align="center">
  <img src="assets/dashboard-v0.1.5.png" alt="WinNUT Client 儀表板" width="100%" />

  <h1 align="center">WinNUT Rust Client</h1>

  <p align="center">
    <strong>透過 NUT 監控 UPS 的現代化、高效能 Windows 桌面程式。</strong>
  </p>

  <p align="center">
    <a href="https://github.com/tatsuyakari1203/WinNUT-Rust-Client/blob/main/LICENSE">
      <img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License" />
    </a>
    <img src="https://img.shields.io/badge/platform-Windows-lightgrey" alt="Platform" />
    <img src="https://img.shields.io/badge/status-stable-green" alt="Status" />
  </p>

  <p align="center">
    以 <b>Tauri v2</b> 與 <b>React</b> 打造，重視外觀、效能與 Windows 原生整合。<br>
    告別笨重又過時的監控工具。
  </p>
</div>

<br />

## ✨ 功能一覽

<table>
  <tr>
    <td width="50%" valign="top">
      <h3>📈 進階分析</h3>
      <p>精確追蹤電力數據：輸入／輸出電壓、負載、電池電量與續航時間，即時更新。</p>
      <p>可縮放的互動圖表，讓你從歷史趨勢了解長期的用電狀況。</p>
    </td>
    <td width="50%">
      <img src="assets/history-v0.1.5.png" alt="歷史資料圖表" width="100%" />
    </td>
  </tr>
  <tr>
    <td width="50%">
      <img src="assets/settings-themes-v0.1.5.png" alt="佈景主題選擇" width="100%" />
    </td>
    <td width="50%" valign="top">
      <h3>🎨 多樣佈景主題</h3>
      <p>內建多款佈景主題，冷冽的 <b>Nord</b>、鮮明的 <b>Dracula</b>、經典的 <b>Catppuccin</b> 任你挑選。</p>
      <ul>
        <li><b>Catppuccin</b> 🐱</li>
        <li><b>Dracula</b> 🧛</li>
        <li><b>Nord</b> ❄️</li>
        <li><b>Monokai</b> 🎨</li>
        <li><b>GitHub Dark</b> 🐙</li>
      </ul>
    </td>
  </tr>
  <tr>
    <td width="50%" valign="top">
      <h3>⚡ 遠端控制與原生整合</h3>
      <p>不只是監看。</p>
      <ul>
        <li><b>遠端控制</b>：直接對 UPS 下指令（電池測試、切換蜂鳴器等）。</li>
        <li><b>原生關機</b>：電池電量達臨界值時，以 Windows 原生機制安全關機、休眠或睡眠。</li>
        <li><b>系統匣</b>：一眼掌握 UPS 狀態。</li>
      </ul>
    </td>
    <td width="50%">
      <img src="assets/remote-control-v0.1.5.png" alt="遠端控制面板" width="100%" />
    </td>
  </tr>
</table>

<br />

## 🚀 安裝

### 下載安裝檔
到 [Releases](https://github.com/ppundsh/WinNUT-Client-RS/releases) 頁面下載最新的 `.msi` 安裝檔。

### 需求
- 一台執行中的 **NUT（Network UPS Tools）** 伺服器，例如 Raspberry Pi、Synology / QNAP NAS 或 Linux 主機。
- **Windows 10/11**（x64）。

---

## ⚙️ 設定說明

<details>
<summary><strong>展開設定步驟</strong></summary>

1.  **連線**：
    - 點齒輪圖示開啟設定。
    - 輸入 NUT 伺服器 IP、連接埠（預設 3493），以及**與伺服器上設定完全相同的 UPS 名稱**（`ups.conf`，例如 QNAP 是 `qnapups`）。可按欄位旁的伺服器按鈕列出伺服器上的 UPS 名稱。
    - 帳號、密碼為選填，只有遠端控制指令（`INSTCMD`）才需要；只監看時留空即可。
    - 按 **「測試並連線」**。程式會實際讀取一次 UPS 資料，UPS 名稱錯誤、存取權限問題或驅動程式資料過期都會立即顯示（標題列也會出現紅色提示）。
2.  **自動關機**：
    - 在設定中啟用「自動關機」。
    - 設定觸發門檻（例如電量低於 30% 或剩餘時間低於 120 秒）。
    - 選擇動作：**關機**、**休眠**或**睡眠**。
    - 觸發時會顯示倒數視窗，可在倒數結束前取消。
3.  **更新**：
    - 到「設定 › 更新」檢查新版本。

</details>

<br />

## 🌐 語言

介面、通知與系統匣選單支援 **繁體中文、简体中文、English、日本語**。
預設跟隨 Windows 語言，可在 **設定 › 一般 › 語言** 切換。

新增語言的方式：
1. 複製 `src/i18n/locales/en.ts` 為 `src/i18n/locales/<語言代碼>.ts`，翻譯其中的字串（保留 `{占位符}`）。
2. 在 `src/i18n/languages.ts`（`Lang`、`LANGUAGES`、`matchLang`）與 `src/i18n/index.ts`（`DICTS`）登記。
3. 執行 `bun run check:i18n`：缺漏、空字串或占位符不一致都會失敗（TypeScript 也會擋下缺少的 key）。

<br />

## 🛠️ 技術架構

<div align="center">
  <img src="https://img.shields.io/badge/Tauri-2.0-FEC00F?style=for-the-badge&logo=tauri&logoColor=black" />
  <img src="https://img.shields.io/badge/React-19-61DAFB?style=for-the-badge&logo=react&logoColor=black" />
  <img src="https://img.shields.io/badge/TypeScript-007ACC?style=for-the-badge&logo=typescript&logoColor=white" />
  <img src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white" />
  <img src="https://img.shields.io/badge/Tailwind_CSS-38B2AC?style=for-the-badge&logo=tailwind-css&logoColor=white" />
</div>

<br />

## 💻 開發

從原始碼建置或參與開發：

```bash
# 取得原始碼
git clone https://github.com/ppundsh/WinNUT-Client-RS.git
cd WinNUT-Client-RS

# 安裝相依套件
bun install

# 啟動開發版
bun run tauri dev
```

### Windows 一鍵建置

雙擊 `build.cmd`，或執行 `scripts\build-windows.ps1`。它會檢查建置工具（Rust MSVC、Bun、
Visual Studio C++ Build Tools、WebView2），依序執行翻譯檢查、TypeScript 型別檢查與
`cargo test`，最後在 `src-tauri\target\release\bundle\` 產生 `.msi` 與 `-setup.exe` 安裝檔。
完整輸出記錄在 `checks.log`。

```powershell
powershell -ExecutionPolicy Bypass -File scripts\build-windows.ps1                 # 檢查 + 測試 + 產生安裝檔
powershell -ExecutionPolicy Bypass -File scripts\build-windows.ps1 -InstallMissing  # 缺少的工具用 winget 自動安裝
powershell -ExecutionPolicy Bypass -File scripts\build-windows.ps1 -Dev             # 改為啟動開發版
```

未設定 `TAURI_SIGNING_PRIVATE_KEY` 時，會略過自動更新用的簽章檔，只產生安裝檔。

### 排查 NUT 連線問題

`src-tauri/src/bin/debug_nut.rs` 會印出伺服器的原始回應：

```bash
cd src-tauri
cargo run --bin debug_nut -- 192.168.1.10 3493 myups            # LIST UPS / LIST VAR / LIST CMD
cargo run --bin debug_nut -- 192.168.1.10 3493 myups user pass  # 需要登入時
```

## 🍴 關於這個 fork

本專案 fork 自 [tatsuyakari1203/WinNUT-Rust-Client](https://github.com/tatsuyakari1203/WinNUT-Rust-Client)，主要變更：

- **修正**：原本只有 UPS 名稱叫 `ups` 時才解析得到資料，其他名稱（例如 `qnapups`）全部顯示空值。現在會回報 NUT 的 `ERR` 錯誤、不再送出空白帳密，並加上連線／讀取逾時避免卡住。
- **修正**：背景輪詢只會有一個（原本每次套用設定都會多開一個）；輪詢錯誤顯示在標題列；「測試並連線」會實際讀取 UPS。
- **新功能**：介面支援繁體中文、简体中文、English、日本語。
- **建置**：Windows 一鍵建置腳本（`build.cmd`）、NUT 用戶端的模擬伺服器測試。
- 程式內的自動更新改為指向本 fork 的 Releases。

## 🤝 參與貢獻與授權

歡迎提交 Pull Request。
以 **MIT 授權** 釋出。
