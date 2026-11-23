//! 麦克风采集。
//!
//! Android 侧用 NDK 的 **AAudio** C API（`libaaudio.so`），通过 `dlopen`
//! 运行时加载：这样编译期不挑 API level，设备低于 26 时也能给出可读的
//! 错误而不是直接崩。比 JNI + Java `AudioRecord` 少一层 Java 代码，也不
//! 需要 `JavaVM` 生命周期管理——nsh 的 Java 层我们改不了，原生是最短路径。
//!
//! 其他平台（Windows/macOS/Linux，本机开发与 `cargo test`）给桩，返回错误，
//! 保证跨端 `cargo check` 能过。
//!
//! 采集是异步的：`start()` 起后台线程录固定时长，结果放进共享槽；
//! UI 侧用 `set_timeout` 轮询 `take()`。这样不阻塞引擎线程。

use std::sync::{Mutex, OnceLock};

/// 采样率，与 `audio_tone` / DTW 一致。
pub const SAMPLE_RATE: u32 = 16_000;

#[derive(Default)]
struct MicState {
    pcm: Option<Vec<f32>>,
    recording: bool,
    error: Option<String>,
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

/// 取走错误信息（取走即清空）。
pub fn take_error() -> Option<String> {
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
fn capture(ms: u32) -> Result<Vec<f32>, String> {
    android::capture(ms)
}

#[cfg(not(target_os = "android"))]
fn capture(_ms: u32) -> Result<Vec<f32>, String> {
    Err("当前平台没有麦克风采集（仅 Android 实装）".to_owned())
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
    use super::SAMPLE_RATE;
    use std::ffi::{CString, c_char, c_void};

    const RTLD_NOW: i32 = 2;
    const AAUDIO_OK: i32 = 0;
    const DIRECTION_INPUT: i32 = 1;
    const FORMAT_PCM_FLOAT: i32 = 2;
    const PERF_LOW_LATENCY: i32 = 12;
    const TIMEOUT_NANOS: i64 = 100_000_000;

    type Handle = *mut c_void;

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

    unsafe fn sym<T: Copy>(lib: Handle, name: &str) -> Option<T> {
        let c = CString::new(name).ok()?;
        let p = unsafe { dlsym(lib, c.as_ptr()) };
        if p.is_null() {
            None
        } else {
            Some(unsafe { std::mem::transmute_copy::<Handle, T>(&p) })
        }
    }

    fn load() -> Result<Api, String> {
        let name = CString::new("libaaudio.so").map_err(|_| "库名非法".to_owned())?;
        let lib = unsafe { dlopen(name.as_ptr(), RTLD_NOW) };
        if lib.is_null() {
            return Err("设备不支持 AAudio（需 Android 8.0 以上）".to_owned());
        }
        unsafe {
            let api = Api {
                create_stream_builder: sym(lib, "AAudio_createStreamBuilder")
                    .ok_or("AAudio 缺少 createStreamBuilder")?,
                set_direction: sym(lib, "AAudioStreamBuilder_setDirection")
                    .ok_or("AAudio 缺少 setDirection")?,
                set_sample_rate: sym(lib, "AAudioStreamBuilder_setSampleRate")
                    .ok_or("AAudio 缺少 setSampleRate")?,
                set_channel_count: sym(lib, "AAudioStreamBuilder_setChannelCount")
                    .ok_or("AAudio 缺少 setChannelCount")?,
                set_format: sym(lib, "AAudioStreamBuilder_setFormat")
                    .ok_or("AAudio 缺少 setFormat")?,
                set_performance_mode: sym(lib, "AAudioStreamBuilder_setPerformanceMode")
                    .ok_or("AAudio 缺少 setPerformanceMode")?,
                open_stream: sym(lib, "AAudioStreamBuilder_openStream")
                    .ok_or("AAudio 缺少 openStream")?,
                delete_builder: sym(lib, "AAudioStreamBuilder_delete")
                    .ok_or("AAudio 缺少 StreamBuilder_delete")?,
                close_stream: sym(lib, "AAudioStream_close")
                    .ok_or("AAudio 缺少 Stream_close")?,
                request_start: sym(lib, "AAudioStream_requestStart")
                    .ok_or("AAudio 缺少 requestStart")?,
                request_stop: sym(lib, "AAudioStream_requestStop")
                    .ok_or("AAudio 缺少 requestStop")?,
                read_frames: sym(lib, "AAudioStream_read")
                    .ok_or("AAudio 缺少 Stream_read")?,
                _lib: lib,
            };
            Ok(api)
        }
    }

    impl Drop for Api {
        fn drop(&mut self) {
            unsafe {
                dlclose(self._lib);
            }
        }
    }

    pub fn capture(ms: u32) -> Result<Vec<f32>, String> {
        let api = load()?;
        unsafe {
            let mut builder: Handle = std::ptr::null_mut();
            if (api.create_stream_builder)(&mut builder) != AAUDIO_OK || builder.is_null() {
                return Err("无法创建 AAudio 流构造器".to_owned());
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
                return Err(format!("打开麦克风失败（{rc}），请确认已授予录音权限"));
            }

            let total = (SAMPLE_RATE as u64 * ms as u64 / 1000) as usize;
            let mut pcm: Vec<f32> = Vec::with_capacity(total);

            if (api.request_start)(stream) != AAUDIO_OK {
                (api.close_stream)(stream);
                return Err("启动录音失败".to_owned());
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
                    let n = (n as usize).min(chunk.len());
                    pcm.extend_from_slice(&chunk[..n]);
                }
            }

            (api.request_stop)(stream);
            (api.close_stream)(stream);

            if pcm.is_empty() {
                return Err("没有采集到音频".to_owned());
            }
            pcm.truncate(total);
            Ok(pcm)
        }
    }
}
