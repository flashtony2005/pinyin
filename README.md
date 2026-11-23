# 拼音拼读（pinyin-pal）

用 Rust + [Native Shell (nsh)](https://nsh.dev) 做的儿童拼音练习工具。

## 功能

- 列出 23 个声母、24 个韵母；点声母先读声母，再点韵母自动拼出音节并朗读。
- 按标准普通话音节表过滤：非法组合（如 `b + ong`）的韵母置灰，点了也不发音。
- 可切换慢速朗读；「音节表」页列出 343 个合法音节，点按可听。
- **四声切换**：拼读结果区有 ā/á/ǎ/à 四个声调按钮，点选后用对应声调发音。
- 真人发音优先：1069 个百度汉语真人音频（分声调），缺失的 40 个音节用本地 TTS 兜底，全部离线打包（共 1435 个文件、压缩后 4.2 MB）。

## 结构

- `src/data/pinyin_data.rs` — 由 `tools/build_table.py` 生成（声母/韵母/音节表/组合矩阵）。
- `src/pinyin.rs` — 纯拼读逻辑（含 7 个单元测试）。
- `src/lib.rs` — nsh UI 与音频播放。
- `tools/build_table.py` — 拼音数据单一来源，生成 Rust 常量与 JSON。
- `tools/gen_audio.py` — 用 macOS `say -v Tingting` + `afconvert` 生成音频。

## 运行

```bash
# Web（推荐）
nsh run . --target web
# 打开 http://127.0.0.1:8790/

# macOS 原生
nsh run .

# 单元测试
cargo test
nsh test
```

### Android（本机 Windows 环境）

工具链都装在 F 盘，不占 C 盘：

| 组件 | 路径 |
| --- | --- |
| nsh CLI 0.5.1 | `F:\nsh\bin\nsh.exe`（`nsh component add android` 已装 android 运行时） |
| Android SDK | `F:\android-sdk`（platform-tools / platforms;android-35 / build-tools;35.0.1） |
| NDK r27d | `F:\android-sdk\ndk\27.3.13750724` |
| JDK 21 | `D:\Tools\jdk-21.0.2`（`keytool` 打包签名要用） |

```bash
# 一条命令：编译 → 打 APK → 装到已连接的真机并启动
export ANDROID_HOME=F:/android-sdk ANDROID_SDK_ROOT=F:/android-sdk \
       ANDROID_NDK_HOME=F:/android-sdk/ndk/27.3.13750724
export PATH="/d/Tools/jdk-21.0.2/bin:/f/android-sdk/platform-tools:$PATH"
/f/nsh/bin/nsh.exe package --target android --abi arm64-v8a --run

# 产物
.nsh/out/拼音拼读.apk
```

- 设备侧：`adb devices` 至少要有一台在线；安装包名 `com.lushaohui.pinyinpal`，启动 Activity 是 `com.longbridge.nsh.AppActivity`。
- Rust 目标：`rustup target add aarch64-linux-android`（已装）。首次编译 nsh 平台层约 14 分钟，之后改代码增量 15 秒左右。
- 灰度：`nsh.json` 里 `icon`/`appearance`/`statusBar`/`android.targetSdk` 生效于 `package`；改完必须重新打包才看得到。
- 没有 release keystore 时用 debug key 签名（安装测试够用，上架需另行签名）。

## iOS（需要 Apple silicon Mac，当前 Windows 机器无法构建）

应用代码是 Rust + nsh 跨端共用，不需要改代码；但 nsh 的 `ios` 运行时只发布给 **macOS on Apple silicon**，且打包要用 Xcode 的 `actool` / `codesign`，所以 iOS 构建**不能在这台 Windows 机器上完成**，必须在 Mac 上跑：

| 前置条件 | 说明 |
| --- | --- |
| Apple silicon Mac（M 系列） | `uname -m` 必须是 `arm64` |
| Xcode + Command Line Tools | `xcode-select --install`；模拟器运行时缺失时 `xcodebuild -downloadPlatform iOS` |
| nsh 0.5.1 | 与 Android 同一版本 |
| Rust targets | `aarch64-apple-ios-sim aarch64-apple-ios` |

```bash
# 在 Mac 上，项目根目录执行
./tools/build-ios.sh simulator

# 真机 / TestFlight：需要开发者签名身份 + 覆盖 com.lushaohui.pinyinpal 的 provisioning profile
security find-identity -v -p codesigning
./tools/build-ios.sh device "Apple Distribution: Your Team (TEAMID)" AppStore.mobileprovision
```

产物：`.nsh/out/拼音拼读.app`（模拟器）或 `.nsh/out/拼音拼读.ipa`（真机）。无签名时 nsh 只产出未签名 `.app`，无法装到 iPhone。最低系统 iOS 15.0。脚本已把 `nsh component add ios`、Rust target 安装和打包串起来；若本机 `nsh` 不在 PATH，用 `NSH_BIN=/path/to/nsh ./tools/build-ios.sh sim`。

## 手机端界面

视觉以浅紫渐变背景 + 白色圆角卡片为主（`tools/make_ui_assets.py` 生成 `assets/bg.png` 与 wave/speaker/clear 图标）。顶部波形图标 + 标题/副标题，右侧胶囊页签：**拼读 / 查表 / 练习 / 知识 / 动画**。

- **拼读**：一屏放下、无滚动——声母 23 个与韵母 24 个各按 6 列分行（4 行），中间一条分隔线；每行 `flex_grow` 等分高度，白色圆角瓦片。独立的「声调」大卡片含喇叭图标、清除按钮与五个彩色声调按钮（一声／二声／三声／四声／轻声，轻声回退第一声音频）。底部播放栏：算式 + ▶播放大按钮 +「例字」+ 规则提示 + 慢速。
  - 点声母（未拼读）→ 提示行显示该声母的顺口溜；拼出音节 → 提示行切换为标调规则 / ü 规则，`例字` 同时显示对应汉字。
  - 「清除」一键清空已选声母/韵母/结果。
- **练习**：五种题型轮换——听音选字、看字选拼音、听音选声母、听音选韵母、听音选声调。干扰项优先按音近/形近分组选取（如 b/p/m/f、an/ang、u/ü、日/曰）。答错自动进入错题库，顶部「错题 N」可切换复习；答对一次减一分，归零后移出错题本，错题库通过 nsh Storage 持久化到设备本地。支持清空错题、跳过与即时发音反馈，完全离线。
- **知识**：6 条拼音规则（声调、ü 书写、整体认读、三拼、平翘舌、前后鼻音）+ 23 个声母顺口溜；每条规则附可点击发音的样例（如「妈 mā / 麻 má / 马 mǎ / 骂 mà」），可滚动。
- **动画**：四幕顺口溜动画课堂，用 nsh 原生 `Animated`（`animated_value` + `Timing` + `bind_opacity` / `bind_translate_y` / `bind_scale`）在 UI 线程逐帧播放，不走引擎重绘：
  1. **四声**：ā/á/ǎ/à 四张卡依次淡入上浮，SVG 调型曲线以 `line_dash_offset` 描线；
  2. **左右手**：b/d/p/q 四格按「左手/右手 × 向下/不向下」依次弹出；
  3. **ü 两点**：左边 j q x 让两点飞走并淡出（写成 u），右边 n l 两点保持；
  4. **介母**：i / u 从上方落进空位，jia、guo 结果块放大高亮。
  底部四幕胶囊可手动切换，每幕字幕对应一句口诀。

例字数据来自 `tools/speak_map.json`（第一声同音字，本为 TTS 兜底生成），由 `build.rs` 编译进二进制。顺口溜与规则文案在 `src/data/mnemonics.rs`，出题逻辑在 `src/quiz.rs`。

## 重新生成数据 / 音频

```bash
python3 tools/build_table.py          # 改音节表后
python3 tools/build_speak_map.py      # 音节表变化后重建「音节→代表汉字」映射
python3 tools/gen_audio.py --force    # 换音色或重新生成音频
```

### 音频来源

1. **百度真人发音**（优先）：`tools/download_baidu_audio.py` 从百度汉语 CDN 下载 1069 个分声调 mp3（URL 规律 `https://hanyu-word-pinyin-short.cdn.bcebos.com/<pinyin><tone>.mp3`，`ü` 写成 `v`），存到 `assets/audio/bd/`，清单在 `tools/baidu_audio.json`。并非所有声调都存在（如 `kou2` 是 404），脚本会先探测再下载；`--reprobe` 可重新探测。
2. **TTS 兜底**：`tools/gen_audio.py` 用 macOS `say -v Tingting` 生成 366 个 m4a，覆盖百度缺失的 40 个音节。直接读拼音会出错（如 `kou` 读成"古"、`ma` 读成英文 "Mom"），故先用 `tools/build_speak_map.py` 从 GB2312 常用字为每个音节选第一声同音字（`tools/speak_map.json`），再交给 TTS。依赖 `pypinyin`（`pip install pypinyin`）。
3. **压缩**：`tools/compress_audio.py` 把全部音频压到 32kbps 单声道，9.54 MB → 4.21 MB。执行前先备份 `assets/audio`（本项目备份在 `assets/audio.orig-20261117/`）。

`build.rs` 把 `tools/baidu_audio.json` 编译成查找表，运行时 `speak()` 优先播百度真人音频，缺失时回退 TTS m4a。

## 本机环境注意事项（macOS）

1. `nsh` 未进 PATH，实际路径 `~/.local/nsh/bin/nsh`（brew 因 Xcode 版本过旧拒绝安装，但 tarball 已解出可用二进制）。
2. Web 构建需两个前置：
   - `rustup target add wasm32-unknown-unknown`
   - 系统 clang 不支持 wasm32 C 编译，且 `~/.cargo/config.toml` 的 `rustc-wrapper = sccache` 会劫持 cc。用：
     ```bash
     CC=/opt/homebrew/opt/llvm@20/bin/clang \
     CXX=/opt/homebrew/opt/llvm@20/bin/clang++ \
     AR=/opt/homebrew/opt/llvm@20/bin/llvm-ar \
     RUSTC_WRAPPER= nsh run . --target web
     ```
3. `nsh run` 占用 8790 端口，重启前先结束旧的 `nsh run` 进程。
4. Android 运行时可 `nsh component add android` 安装；真机验证已在 Windows 机器上完成（见上面「Android（本机 Windows 环境）」）。应用代码为一套 Rust 跨端共用，Android 不需要单独的工程或改动。
