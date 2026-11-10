# adb-wrapper

Rust 实现的 `adb` 透明代理：多设备连接时自动选择设备并注入 `-s <serial>`，单设备或已指定设备时行为与官方 adb 一致。

> 本 crate 是 [akbox](../..) workspace 成员，位于 `crates/adb-wrapper/`。

## 构建

需要 [Rust](https://rustup.rs/) 1.70+：

```bash
cargo build --release --manifest-path crates/adb-wrapper/Cargo.toml
```

产物：`target/release/adb.exe`（Windows）或 `target/release/adb`（Linux/macOS）。

## 推荐使用方式

推荐把本工具当作 PATH 前置的透明代理使用：

```text
用户继续输入 adb ...
    ↓
PATH 先找到 adb-wrapper
    ↓
adb-wrapper 调用官方 adb
```

**不要替换 Android SDK 自带的 `adb.exe`。** 更稳妥的方式是把 wrapper 放到独立目录，并让该目录在 `PATH` 中排在 SDK `platform-tools` 前面。

### 1. 准备 wrapper 目录

Windows 示例：

```powershell
New-Item -ItemType Directory -Force C:\Tools\adb-wrapper | Out-Null
Copy-Item target\release\adb.exe C:\Tools\adb-wrapper\
```

Linux / macOS 示例：

```bash
mkdir -p ~/bin/adb-wrapper
cp target/release/adb ~/bin/adb-wrapper/adb
chmod +x ~/bin/adb-wrapper/adb
```

### 2. 指定官方 adb 路径

优先推荐使用 `REAL_ADB` 指向 Android SDK 里的官方 adb。

Windows PowerShell：

```powershell
$env:REAL_ADB = "C:\Android\platform-tools\adb.exe"
$env:PATH = "C:\Tools\adb-wrapper;$env:PATH"
```

Linux / macOS：

```bash
export REAL_ADB="$HOME/Android/Sdk/platform-tools/adb"
export PATH="$HOME/bin/adb-wrapper:$PATH"
```

需要长期生效时，把这两行写入 PowerShell Profile、`.bashrc`、`.zshrc` 或对应 shell 配置。

### 3. 验证是否接管成功

配置后先运行：

```bash
adb --wrapper-info
```

重点检查：

- `adb-wrapper: active`
- `real adb` 指向官方 SDK adb
- `recursion risk: no`

如果 `real adb` 未找到，设置 `REAL_ADB` 或使用下面的同目录 `adb-real` 方式。

### 4. 日常使用

安装完成后仍然按原来的习惯输入 adb：

```bash
adb shell
adb push app.apk /data/local/tmp/
adb logcat
```

行为：

- **单设备**：直接透传给官方 adb。
- **多设备 + 交互终端**：首次弹出设备选择列表并自动注入 `-s <serial>`；同一终端会话内后续命令复用该选择，不再重复弹窗。
- **多设备 + 脚本/CI/非交互**：不弹选择器，直接报错，要求显式写 `adb -s <serial> ...` 或设置 `ANDROID_SERIAL`。
- **已经指定 `-s` / `-d` / `-e` 或设置 `ANDROID_SERIAL`**：不干预，直接透传。

### 同目录 `adb-real`（便携）

将官方 `adb` 重命名后与 wrapper 放在一起：

```text
adb-wrapper/
  adb.exe        # wrapper
  adb-real.exe   # 官方 adb
```

无需设置 `REAL_ADB`。

## 配置文件

| 平台    | 路径 |
|---------|------|
| Windows | `%APPDATA%\adb-wrapper\config.toml` |
| Linux   | `~/.config/adb-wrapper/config.toml` |
| macOS   | `~/Library/Application Support/adb-wrapper/config.toml` |

示例：

```toml
real_adb = "C:/Android/platform-tools/adb.exe"
prefer_last_device = true    # 在当前终端会话内记住上次选择
remember_ttl_secs = 43200    # 记忆有效期（秒），0 表示不过期
show_model = true            # 选择列表显示型号
selector = "inquire"         # inquire | skim | fzf
```

**真实 adb 查找优先级：** `REAL_ADB` 环境变量 → 配置 `real_adb` → 同目录 `adb-real`（不从 PATH 查找，避免递归调用 wrapper）。

## 设备记忆（多设备）

选择结果按**终端会话**记忆，而不是全局共享：

1. `ADB_WRAPPER_SESSION` 环境变量（显式作用域，适合脚本/CI）；
2. 平台终端会话标识（Unix 会话 ID / Windows 控制台窗口）；
3. 工作目录兜底。

因此同一终端里连续执行多条 `adb` 命令只会选一次；不同终端并行运行不同脚本时互不干扰。记忆保存在缓存目录的 `state.json`，默认 12 小时后失效，并在每次使用前校验设备是否仍在线。

需要重新选择时：

```bash
adb --pick shell     # 强制弹出选择器并覆盖当前会话记忆
adb -s <serial> ...  # 一次性指定，不影响记忆
```

清除当前会话记忆：

```bash
adb --forget
```

脚本或 CI 中也可以完全绕过记忆，直接使用官方 adb 支持的 `ANDROID_SERIAL`：

```bash
ANDROID_SERIAL=123456F adb shell
```

## 常用命令

强制重新选择设备：

```bash
ADB_WRAPPER_PICK=1 adb shell
adb --pick shell
adb --forget
```

**自检 / 排查 PATH 与配置：**

```bash
adb --wrapper-info
```

示例输出：

```text
adb-wrapper: active
wrapper version: 0.1.0
wrapper path: C:\Tools\adb-wrapper\adb.exe
real adb: C:\Android\platform-tools\adb.exe (via REAL_ADB)
REAL_ADB env: C:\Android\platform-tools\adb.exe
  candidate [REAL_ADB]: C:\Android\platform-tools\adb.exe (ok)
  candidate [adb-real (sibling)]: C:\Tools\adb-wrapper\adb-real.exe (missing)
stdin tty: true
stdout tty: true
recursion risk: no
```

用于确认：是否走了 wrapper、`REAL_ADB` 指向、真实 adb 能否解析、当前是否 TTY、是否存在递归调用风险。

管理类命令（`devices`、`version`、`help`、`start-server` 等）始终透传。

## Shell 补全

```bash
# Bash
adb completions bash >> ~/.bashrc

# Zsh / Fish / PowerShell
adb completions zsh   >> ~/.zshrc
adb completions fish  >> ~/.config/fish/completions/adb.fish
adb completions powershell >> $PROFILE
```

`completions` 为 wrapper 内置子命令，无需配置 `REAL_ADB`。

## 选择器模式

| `selector` | 说明 |
|------------|------|
| `inquire`（默认） | 方向键列表 |
| `skim` | 内置模糊搜索 |
| `fzf` | 调用外部 `fzf`，失败时依次回退 skim → inquire |

## 常见问题

**`adb-wrapper: real adb not found`**

设置 `REAL_ADB`，或在 wrapper 同目录放置 `adb-real`（Windows 为 `adb-real.exe`）。

**CI / 脚本卡住或报错**

多设备且未指定 `-s` 时，非 TTY 环境会主动失败，请写 `adb -s <serial> ...` 或设置 `ANDROID_SERIAL=<serial>`。

**无效 `config.toml`**

启动时会告警并回退默认配置，请检查 TOML 语法。

## 开发

```bash
cargo test --manifest-path crates/adb-wrapper/Cargo.toml          # 单元测试 + 集成测试
cargo build --release --manifest-path crates/adb-wrapper/Cargo.toml
```

## 许可

见仓库根目录（若未单独声明，以项目所有者为准）。
