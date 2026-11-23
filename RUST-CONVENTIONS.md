# Rust 工程约束（pinyin-pal）

依据 `agent-kit` 的 Rust 指南（2026-11-09），按本项目的实际情况裁剪。
原则：**请求优先，其次项目约定；不为不需要的场景加硬化，不重写能跑的代码。**

本项目是单 crate（`pinyin-pal`），无 workspace、无 async runtime、无 serde
依赖，所以原指南里 workspace 拆分、依赖方向、tokio 取消安全、serde 版本兼容
等章节不适用，下文只保留真正生效的条目。

## 错误

- 调用方需要分支的错误，返回**自己的错误枚举**，不返回 `io::Result`、
  `String`、`Box<dyn Error>` 或依赖的错类型。
- 内部**不要**把原因拍平成文本（`e.to_string()`、`Result<_, String>`）；分支
  判断靠类型，不靠匹配消息。
- 面向用户的文案在边界层映射，内部原因留在 `Debug` / 日志。

现状：`mic::MicError` 是范例——`OpenFailed(-889)` 专门表达"没权限"，界面据此
提示去设置，而不是解析字符串。

## unsafe

- `unsafe` 块必须有 `// SAFETY:` 说明成立的前提；由
  `[lints.clippy] undocumented_unsafe_blocks = "deny"` 强制。
- 本项目 unsafe 只有一处来源：`src/mic.rs` 的 AAudio FFI（`dlopen` /
  `dlsym` / 函数指针转换）。新增 unsafe 前先问能不能用安全抽象替代。

## 类型与零成本抽象

- 静态分发优先：封闭集合用 `enum`，`dyn` 只留给开放注册表与测试缝。
- 不给自己枚举写 `_` 分支；新增变体时让编译器报错。
- 有不变量的值（id、尺寸、上限）用私有字段 newtype + 一个可失败构造器。
- 变换常常原样返回时用 `Cow<'_, str>`；大量共享的不可变数据用 `Arc`。
- 逐帧/逐项循环复用缓冲区（`clear()`、`mem::take`），别每轮分配。

现状：`audio_tone` 与 `dtw` 的逐帧循环已复用 `Vec` 并只做一次分配；
`MfccConfig` 是纯值配置，无需 newtype。

## 并发与生命周期（nsh 特有）

- **`Signal<T>` 基于 `Rc<RefCell<_>>`，不是线程安全的**：绝不能在
  `std::thread` 里写它，否则是未定义行为级别的错误用法。
- 后台线程的结果走共享槽（`OnceLock<Mutex<_>>`），UI 侧用 `set_timeout`
  轮询取走，再在引擎线程写 `Signal`。`mic::start` / `mic::take` 是范例。
- `spawn` / `set_timeout` 返回的 `Spawned` **一 drop 就取消**；需要它活到
  触发时，显式 `std::mem::forget`（本项目已知的极小句柄泄漏，可接受）。
- 长跑循环里从外部进来的数据要有上限（队列长度、录音时长）。

## 进程、文件与 panic

- `Drop` 只做尽力而为、同步、不阻塞、不 panic 的清理；能失败的释放另外提供
  `close(self) -> Result`。
- 状态文件原子替换：同目录临时文件 → 写 → `sync_all` → rename。
- `expect` / `unreachable!` 的消息要说明**排除失败的不变量**，而不是复述症状。
- 长期运行代码优先 `get` / `split_first` / `split_at_checked` / 切片模式，
  少用裸索引（数值算法例外，见下）。

## 测试与 lint

已启用（`Cargo.toml` + `src/lib.rs`）：

```toml
[lints.clippy]
undocumented_unsafe_blocks = "deny"
```

```rust
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]
```

- 测试里允许 `unwrap` / `expect`：夹具必须成立，失败就该炸。
- 压 lint 用 `#[expect(lint, reason = "...")]` 打在最小项上，**不用**裸
  `#[allow]`，不用 crate 级豁免。
- 时间相关代码用注入时钟，不真睡。

### 明确不启用

| 规则 | 原因 |
|---|---|
| `clippy::indexing_slicing` | `dtw.rs` 的 FFT / 梅尔滤波 / DTW 全是边界明确的数值索引，硬改成 `get().unwrap_or` 会显著降低可读性且更慢。原指南也允许按需取舍。 |
| `clippy::too_many_arguments` | 命中的是 `pair_card`(8) / `nasal_card`(11) 这类 UI 构造函数，拆结构体只会让 nsh 的调用点更长。 |
| `needless_range_loop` | `dtw.rs` 的循环同时索引多个数组（`mat[c][m]`、`re/im`），迭代器改写反而绕。 |
| `disallowed-methods` | 项目里没有"必须走包装器"的原始 API。 |

## 当前已知偏差（记录，不擅自改）

- `clippy` 余下约 20 条风格警告：`redundant_closure`（nsh 的 `when` /
  `otherwise` 需要闭包而非函数名）、`collapsible_if`、`manual_contains`、
  `useless_vec` 等。均不影响正确性，按"不重写能跑的代码"保留。
- `MicError` 已类型化，但 `RecTest::Failed(String)` 仍是字符串——它是 UI
  展示态而非 API 错误，保留字符串是有意为之。
