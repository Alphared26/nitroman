//! High-performance NVIDIA Management Library (NVML) client for real-time GPU telemetry.

use std::ffi::c_void;

#[repr(C)]
struct NvmlUtilization {
    pub gpu: u32,
    pub memory: u32,
}

#[repr(C)]
struct NvmlMemory {
    pub total: u64,
    pub free: u64,
    pub used: u64,
}

#[derive(Default, Debug, Clone)]
pub struct NvmlGpuStats {
    pub temp_c: Option<f64>,
    pub load_pct: Option<f64>,
    pub clock_mhz: Option<u32>,
    pub vram_clock_mhz: Option<u32>,
    pub vram_used_mb: Option<u32>,
    pub vram_total_mb: Option<u32>,
}

#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn LoadLibraryA(lp_lib_file_name: *const u8) -> *mut c_void;
    fn GetProcAddress(h_module: *mut c_void, lp_proc_name: *const u8) -> *mut c_void;
    fn FreeLibrary(h_module: *mut c_void) -> i32;
}

#[cfg(windows)]
pub fn query_nvml_stats() -> Option<NvmlGpuStats> {
    unsafe {
        let h_mod = LoadLibraryA(b"nvml.dll\0".as_ptr());
        if h_mod.is_null() {
            return None;
        }

        let init_fn: Option<extern "system" fn() -> i32> =
            std::mem::transmute(GetProcAddress(h_mod, b"nvmlInit_v2\0".as_ptr()));
        let get_dev_fn: Option<extern "system" fn(u32, *mut *mut c_void) -> i32> =
            std::mem::transmute(GetProcAddress(h_mod, b"nvmlDeviceGetHandleByIndex_v2\0".as_ptr()));
        let get_util_fn: Option<extern "system" fn(*mut c_void, *mut NvmlUtilization) -> i32> =
            std::mem::transmute(GetProcAddress(h_mod, b"nvmlDeviceGetUtilizationRates\0".as_ptr()));
        let get_temp_fn: Option<extern "system" fn(*mut c_void, i32, *mut u32) -> i32> =
            std::mem::transmute(GetProcAddress(h_mod, b"nvmlDeviceGetTemperature\0".as_ptr()));
        let get_clock_fn: Option<extern "system" fn(*mut c_void, i32, *mut u32) -> i32> =
            std::mem::transmute(GetProcAddress(h_mod, b"nvmlDeviceGetClockInfo\0".as_ptr()));
        let get_mem_fn: Option<extern "system" fn(*mut c_void, *mut NvmlMemory) -> i32> =
            std::mem::transmute(GetProcAddress(h_mod, b"nvmlDeviceGetMemoryInfo\0".as_ptr()));
        let shutdown_fn: Option<extern "system" fn() -> i32> =
            std::mem::transmute(GetProcAddress(h_mod, b"nvmlShutdown\0".as_ptr()));

        let mut stats = NvmlGpuStats::default();

        if let (Some(init), Some(get_dev), Some(get_util), Some(get_temp), Some(get_clock), Some(get_mem), Some(shutdown)) =
            (init_fn, get_dev_fn, get_util_fn, get_temp_fn, get_clock_fn, get_mem_fn, shutdown_fn)
        {
            if init() == 0 {
                let mut dev: *mut c_void = std::ptr::null_mut();
                if get_dev(0, &mut dev) == 0 && !dev.is_null() {
                    let mut util = NvmlUtilization { gpu: 0, memory: 0 };
                    if get_util(dev, &mut util) == 0 {
                        stats.load_pct = Some(util.gpu as f64);
                    }

                    let mut temp: u32 = 0;
                    if get_temp(dev, 0, &mut temp) == 0 && temp > 0 && temp < 120 {
                        stats.temp_c = Some(temp as f64);
                    }

                    let mut gfx_clock: u32 = 0;
                    if get_clock(dev, 0, &mut gfx_clock) == 0 {
                        stats.clock_mhz = Some(gfx_clock);
                    }

                    let mut mem_clock: u32 = 0;
                    if get_clock(dev, 2, &mut mem_clock) == 0 {
                        stats.vram_clock_mhz = Some(mem_clock);
                    }

                    let mut mem = NvmlMemory { total: 0, free: 0, used: 0 };
                    if get_mem(dev, &mut mem) == 0 {
                        stats.vram_used_mb = Some((mem.used / (1024 * 1024)) as u32);
                        stats.vram_total_mb = Some((mem.total / (1024 * 1024)) as u32);
                    }
                }
                let _ = shutdown();
            }
        }

        let _ = FreeLibrary(h_mod);

        Some(stats)
    }
}

#[cfg(not(windows))]
pub fn query_nvml_stats() -> Option<NvmlGpuStats> {
    None
}
