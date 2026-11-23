//! 麦克风采集。
//!
//! Android 侧用 NDK 的 **AAudio** C API（`libaaudio.so`），通过 `dlopen`
//! 运行时加载：这样编译期不挑 API level，设备低于 26 时也能给出可读的
//! 错误而不是直接崩。比 JNI + Java `AudioRecord` 少一层 Java 代码，也不
//! 需要 `JavaVM` 生命周期管理——nsh 的 Java 层我们改不了，原生是最短路径。
//!
//! 其他平台（Windows/macOS/Linux，本机开发与 `cargo test`）给桩，返回
//! [`MicError::UnsupportedPlatform`]，保证跨端 `cargo check` 能过。
//!
//! 采集是异步的：`start()` 起后台线程录固定时长，结果放进共享槽；
//! UI 侧用 `set_timeout` 轮询 `take()`。这样不阻塞引擎线程。

use std::fmt;
use std::sync::{Mutex, OnceLock};

/// 采样率，与 `audio_tone` / DTW 一致。
pub const SAMPLE_RATE: u32 = 16_000;

/// 采集失败的原因。
///
/// 调用方要对不同原因给不同提示（重试 / 去设置授权 / 换设备），所以返回
/// 类型而不是字符串——分支判断靠类型，不靠解析文案。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MicError {
    /// 当前构建没有麦克风后端（桌面与测试）。
    UnsupportedPlatform,
    /// 设备加载不到 AAudio：Android 8.0（API 26）以下。
    AaudioUnavailable,
    /// `libaaudio.so` 存在但缺少预期符号，通常是系统库被裁剪。
    MissingSymbol(&'static str),
    /// 打开输入流失败；`code` 是 AAudio 结果码，`-889` 表示没拿到录音权限。
    OpenFailed(i32),
    /// 流已打开但 `requestStart` 失败。
    StartFailed,
    /// 录完了但一个采样都没有。
    EmptyCapture,
}

impl MicError {
    /// 给界面用的中文提示。内部原因留在 `Debug`，这里只讲用户能做的动作。
    pub fn message(self) -> &'static str {
        match self {
            MicError::UnsupportedPlatform => "当前设备不支持录音",
            MicError::AaudioUnavailable => "系统版本过低（需 Android 8 以上）",
            MicError::MissingSymbol(_) => "系统音频库不完整，无法录音",
            MicError::OpenFailed(-889) => "没有麦克风权限，请到设置里允许",
            MicError::OpenFailed(_) => "麦克风被占用，稍后再试",
            MicError::StartFailed => "启动录音失败，再试一次",
            MicError::EmptyCapture => "没有采集到声音，靠近一点再读",
        }
    }
}

impl fmt::Display for MicError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MicError::UnsupportedPlatform => write!(f, "no microphone backend on this platform"),
            MicError::AaudioUnavailable => write!(f, "AAudio unavailable (needs API 26)"),
            MicError::MissingSymbol(sym) => write!(f, "libaaudio.so lacks {sym}"),
            MicError::OpenFailed(code) => write!(f, "AAudioStreamBuilder_openStream returned {code}"),
            MicError::StartFailed => write!(f, "AAudioStream_requestStart failed"),
            MicError::EmptyCapture => write!(f, "capture produced no samples"),
        }
    }
}

impl std::error::Error for MicError {}

#[derive(Default)]
struct MicState {
    pcm: Option<Vec<f32>>,
    recording: bool,
    error: Option<MicError>,
}

fn state() -> &'static Mutex<MicState> {
    static S: OnceLock<Mutex<MicState>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(MicState::default()))
}

/// 是否正在录音。
pub fn is_recording() -> bool {
    state().lock().map(|s| s.recording).unwrap_or(false)
}

/// 取走最近一次录音（取走即清空，故只读一次）。
pub fn take() -> Option<Vec<f32>> {
    state().lock().ok().and_then(|mut s| s.pcm.take())
}

/// 取走错误（取走即清空）。
pub fn take_error() -> Option<MicError> {
    state().lock().ok().and_then(|mut s| s.error.take())
}

/// 开始录制 `ms` 毫秒。已在录音时返回 `false`。
pub fn start(ms: u32) -> bool {
    {
        let mut s = match state().lock() {
            Ok(s) => s,
            Err(_) => return false,
        };
        if s.recording {
            return false;
        }
        s.recording = true;
        s.pcm = None;
        s.error = None;
    }
    std::thread::spawn(move || {
        let result = capture(ms);
        let mut s = match state().lock() {
            Ok(s) => s,
            Err(_) => return,
        };
        s.recording = false;
        match result {
            Ok(pcm) => s.pcm = Some(pcm),
            Err(e) => s.error = Some(e),
        }
    });
    true
}

#[cfg(target_os = "android")]
fn capture(ms: u32) -> Result<Vec<f32>, MicError> {
    android::capture(ms)
}

#[cfg(not(target_os = "android"))]
fn capture(_ms: u32) -> Result<Vec<f32>, MicError> {
    Err(MicError::UnsupportedPlatform)
}

/// 桌面/测试用：把一段已有 PCM 当作"刚刚录完"，便于在非 Android 上
/// 跑通分析链路。
pub fn inject(pcm: Vec<f32>) {
    if let Ok(mut s) = state().lock() {
        s.recording = false;
        s.error = None;
        s.pcm = Some(pcm);
    }
}

#[cfg(target_os = "android")]
mod android {
    use super::{MicError, SAMPLE_RATE};
    use std::ffi::{CString, c_char, c_void};

    const RTLD_NOW: i32 = 2;
    const AAUDIO_OK: i32 = 0;
    const DIRECTION_INPUT: i32 = 1;
    const FORMAT_PCM_FLOAT: i32 = 2;
    const PERF_LOW_LATENCY: i32 = 12;
    const TIMEOUT_NANOS: i64 = 100_000_000;

    type Handle = *mut c_void;

    // SAFETY: 三个函数都是 dlfcn 的 ABI 稳定导出，签名与 <dlfcn.h> 一致；
    // 入参出参都是裸指针，不跨 FFI 传递 Rust 所有权。
    #[link(name = "dl")]
    unsafe extern "C" {
        fn dlopen(filename: *const c_char, flag: i32) -> Handle;
        fn dlsym(handle: Handle, symbol: *const c_char) -> Handle;
        fn dlclose(handle: Handle) -> i32;
    }

    type CreateStreamBuilder = unsafe extern "C" fn(*mut Handle) -> i32;
    type SetI32 = unsafe extern "C" fn(Handle, i32);
    type OpenStream = unsafe extern "C" fn(Handle, *mut Handle) -> i32;
    type DeleteBuilder = unsafe extern "C" fn(Handle);
    type CloseStream = unsafe extern "C" fn(Handle) -> i32;
    type RequestState = unsafe extern "C" fn(Handle) -> i32;
    type ReadFrames = unsafe extern "C" fn(Handle, *mut c_void, i32, i64) -> i32;

    /// AAudio 的一组函数指针；`load()` 一次解析，之后复用。
    struct Api {
        create_stream_builder: CreateStreamBuilder,
        set_direction: SetI32,
        set_sample_rate: SetI32,
        set_channel_count: SetI32,
        set_format: SetI32,
        set_performance_mode: SetI32,
        open_stream: OpenStream,
        delete_builder: DeleteBuilder,
        close_stream: CloseStream,
        request_start: RequestState,
        request_stop: RequestState,
        read_frames: ReadFrames,
        _lib: Handle,
    }

    /// 从已加载的库取一个符号并转成函数指针。
    ///
    /// # Safety
    /// `lib` 必须是一个尚未 `dlclose` 的 `dlopen` 句柄；`T` 必须是该符号
    /// 真实的 C 函数签名，且 `T` 与 `*mut c_void` 同尺寸。
    unsafe fn sym<T: Copy>(lib: Handle, name: &str) -> Option<T> {
        let c = CString::new(name).ok()?;
        // SAFETY: 由调用者保证 lib 有效；c 是本函数的存活 CString；
        // dlsym 只读这两个参数。
        let p = unsafe { dlsym(lib, c.as_ptr()) };
        if p.is_null() {
            None
        } else {
            // SAFETY: 调用者保证 T 是该符号的真实签名；函数指针与数据指针
            // 在目标平台（aarch64-linux-android）同尺寸，transmute_copy 只
            // 做位复制、不产生引用。
            Some(unsafe { std::mem::transmute_copy::<Handle, T>(&p) })
        }
    }

    fn load() -> Result<Api, MicError> {
        let name = CString::new("libaaudio.so").map_err(|_| MicError::MissingSymbol("libaaudio.so"))?;
        // SAFETY: name 是合法的 NUL 结尾字符串；RTLD_NOW 是有效标志。
        // 返回的句柄在 Api 里持有，Drop 时 dlclose。
        let lib = unsafe { dlopen(name.as_ptr(), RTLD_NOW) };
        if lib.is_null() {
            return Err(MicError::AaudioUnavailable);
        }
        // SAFETY: lib 刚由 dlopen 得到且未关闭；每个符号名都以 NUL 结尾的
        // CString 传入，解析结果按 AAudio 的真实签名声明（见上方 type 别名）。
        unsafe {
            let api = Api {
                create_stream_builder: sym(lib, "AAudio_createStreamBuilder")
                    .ok_or(MicError::MissingSymbol("AAudio_createStreamBuilder"))?,
                set_direction: sym(lib, "AAudioStreamBuilder_setDirection")
                    .ok_or(MicError::MissingSymbol("AAudioStreamBuilder_setDirection"))?,
                set_sample_rate: sym(lib, "AAudioStreamBuilder_setSampleRate")
                    .ok_or(MicError::MissingSymbol("AAudioStreamBuilder_setSampleRate"))?,
                set_channel_count: sym(lib, "AAudioStreamBuilder_setChannelCount")
                    .ok_or(MicError::MissingSymbol("AAudioStreamBuilder_setChannelCount"))?,
                set_format: sym(lib, "AAudioStreamBuilder_setFormat")
                    .ok_or(MicError::MissingSymbol("AAudioStreamBuilder_setFormat"))?,
                set_performance_mode: sym(lib, "AAudioStreamBuilder_setPerformanceMode")
                    .ok_or(MicError::MissingSymbol("AAudioStreamBuilder_setPerformanceMode"))?,
                open_stream: sym(lib, "AAudioStreamBuilder_openStream")
                    .ok_or(MicError::MissingSymbol("AAudioStreamBuilder_openStream"))?,
                delete_builder: sym(lib, "AAudioStreamBuilder_delete")
                    .ok_or(MicError::MissingSymbol("AAudioStreamBuilder_delete"))?,
                close_stream: sym(lib, "AAudioStream_close")
                    .ok_or(MicError::MissingSymbol("AAudioStream_close"))?,
                request_start: sym(lib, "AAudioStream_requestStart")
                    .ok_or(MicError::MissingSymbol("AAudioStream_requestStart"))?,
                request_stop: sym(lib, "AAudioStream_requestStop")
                    .ok_or(MicError::MissingSymbol("AAudioStream_requestStop"))?,
                read_frames: sym(lib, "AAudioStream_read")
                    .ok_or(MicError::MissingSymbol("AAudioStream_read"))?,
                _lib: lib,
            };
            Ok(api)
        }
    }

    impl Drop for Api {
        fn drop(&mut self) {
            // SAFETY: _lib 由本结构独占持有，load() 保证它来自 dlopen 且
            // 只在这里关闭一次。
            unsafe {
                dlclose(self._lib);
            }
        }
    }

    pub fn capture(ms: u32) -> Result<Vec<f32>, MicError> {
        let api = load()?;
        // SAFETY: 下面每个调用都满足 AAudio 的前置条件——builder/stream 由
        // 前一步成功返回且非空，句柄在 close 前只使用一次；read_frames 的
        // 缓冲区是本函数持有的 Vec<f32>，长度按帧数传入，读写范围不越界。
        unsafe {
            let mut builder: Handle = std::ptr::null_mut();
            if (api.create_stream_builder)(&mut builder) != AAUDIO_OK || builder.is_null() {
                return Err(MicError::MissingSymbol("AAudio_createStreamBuilder"));
            }
            (api.set_direction)(builder, DIRECTION_INPUT);
            (api.set_sample_rate)(builder, SAMPLE_RATE as i32);
            (api.set_channel_count)(builder, 1);
            (api.set_format)(builder, FORMAT_PCM_FLOAT);
            (api.set_performance_mode)(builder, PERF_LOW_LATENCY);

            let mut stream: Handle = std::ptr::null_mut();
            let rc = (api.open_stream)(builder, &mut stream);
            (api.delete_builder)(builder);
            if rc != AAUDIO_OK || stream.is_null() {
                return Err(MicError::OpenFailed(rc));
            }

            let total = (SAMPLE_RATE as u64 * ms as u64 / 1000) as usize;
            let mut pcm: Vec<f32> = Vec::with_capacity(total);

            if (api.request_start)(stream) != AAUDIO_OK {
                (api.close_stream)(stream);
                return Err(MicError::StartFailed);
            }

            let mut chunk = vec![0f32; 2048];
            while pcm.len() < total {
                let n = (api.read_frames)(
                    stream,
                    chunk.as_mut_ptr() as *mut c_void,
                    chunk.len() as i32,
                    TIMEOUT_NANOS,
                );
                if n < 0 {
                    break;
                }
                if n > 0 {
                    // AAudio 单声道、float32：返回帧数即采样数，不会超出 chunk。
                    let n = (n as usize).min(chunk.len());
                    pcm.extend_from_slice(&chunk[..n]);
                }
            }

            (api.request_stop)(stream);
            (api.close_stream)(stream);

            if pcm.is_empty() {
                return Err(MicError::EmptyCapture);
            }
            pcm.truncate(total);
            Ok(pcm)
        }
    }
}
