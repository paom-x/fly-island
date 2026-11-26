//! Graphics load and package temperature, read through Windows' performance counters (PDH).
//!
//! Both are optional: the counters exist on most machines but not all (virtual machines and older
//! drivers have no `GPU Engine` instances, and thermal zones are often absent on desktops whose
//! firmware does not publish them). A missing counter makes the reading `None`, so the island
//! hides the metric instead of showing a zero it invented.
//!
//! PDH keeps one query open for both counters. Its percentage counters are computed from the
//! difference between two collections, so the first reading after start-up is a warm-up and reads
//! zero; the front end polls every second or two, so that is not visible.

use std::sync::Mutex;

use windows::{
    core::{HSTRING, PCWSTR},
    Win32::System::Performance::{
        PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterArrayW,
        PdhOpenQueryW, PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE, PDH_HCOUNTER, PDH_HQUERY,
    },
};

/// Every graphics engine's utilization, summed the way Task Manager shows the engines.
const GPU_PATH: &str = r"\GPU Engine(*)\Utilization Percentage";
/// ACPI thermal zone temperature, in tenths of a degree Kelvin.
const THERMAL_PATH: &str = r"\Thermal Zone Information(*)\Temperature";

/// `PDH_MORE_DATA`: the buffer was too small and its required size was written back.
const PDH_MORE_DATA: u32 = 0x8000_07D2;
/// `PDH_CSTATUS_VALID_DATA`: the counter produced a usable value.
const PDH_CSTATUS_VALID_DATA: u32 = 0x0000_0000;

struct Counters {
    query: PDH_HQUERY,
    gpu: PDH_HCOUNTER,
    thermal: PDH_HCOUNTER,
}

// PDH hands back opaque handles, not pointers into Rust-owned memory, and this module only touches
// them while holding the mutex below.
// SAFETY: PDH_HQUERY and PDH_HCOUNTER are opaque handles accepted by PDH on any thread; no thread
// other than the one holding `COUNTERS` can reach them.
unsafe impl Send for Counters {}

/// Closing the query releases every counter that was added to it.
impl Drop for Counters {
    fn drop(&mut self) {
        // SAFETY: this is the query opened in `Counters::open` and never given to anyone else.
        unsafe { PdhCloseQuery(self.query) };
    }
}

impl Counters {
    fn open() -> Option<Self> {
        let mut query = PDH_HQUERY(std::ptr::null_mut());
        // SAFETY: a local out-parameter; a null data source means "the local machine".
        if unsafe { PdhOpenQueryW(PCWSTR::null(), 0, &mut query) } != 0 {
            return None;
        }
        let mut counters = Self {
            query,
            gpu: PDH_HCOUNTER(std::ptr::null_mut()),
            thermal: PDH_HCOUNTER(std::ptr::null_mut()),
        };
        // English counter names: the localized ones differ per Windows language, and the plain
        // `PdhAddCounter` would be at their mercy. A counter this machine lacks simply fails to
        // add, and reading it later reports nothing.
        let (gpu, thermal) = (HSTRING::from(GPU_PATH), HSTRING::from(THERMAL_PATH));
        // SAFETY: the query is open and each counter out-parameter points at a local field; both
        // paths outlive the call.
        unsafe {
            PdhAddEnglishCounterW(counters.query, PCWSTR(gpu.as_ptr()), 0, &mut counters.gpu);
            PdhAddEnglishCounterW(counters.query, PCWSTR(thermal.as_ptr()), 0, &mut counters.thermal);
        }
        Some(counters)
    }
}

static COUNTERS: Mutex<Option<Counters>> = Mutex::new(None);

/// Values of every instance of `counter`, skipping instances whose collection failed.
fn instance_values(counter: PDH_HCOUNTER) -> Vec<f64> {
    let mut size = 0u32;
    let mut count = 0u32;
    // SAFETY: the documented two-pass probe — with no buffer, PDH only reports the size it needs.
    let status =
        unsafe { PdhGetFormattedCounterArrayW(counter, PDH_FMT_DOUBLE, &mut size, &mut count, None) };
    if status != PDH_MORE_DATA || size == 0 || count == 0 {
        return Vec::new();
    }
    // The size PDH reports covers the instance names as well as the items, so it is allocated as
    // bytes, rounded up to the items' alignment; allocating `count` items would under-allocate and
    // PDH would write past the end of it.
    let mut buffer: Vec<u64> = vec![0; (size as usize).div_ceil(std::mem::size_of::<u64>())];
    // SAFETY: the buffer is at least `size` bytes — what PDH asked for — and 8-byte aligned like
    // PDH_FMT_COUNTERVALUE_ITEM_W requires.
    let status = unsafe {
        PdhGetFormattedCounterArrayW(
            counter,
            PDH_FMT_DOUBLE,
            &mut size,
            &mut count,
            Some(buffer.as_mut_ptr() as *mut PDH_FMT_COUNTERVALUE_ITEM_W),
        )
    };
    if status != 0 || (count as usize) * std::mem::size_of::<PDH_FMT_COUNTERVALUE_ITEM_W>() > buffer.len() * std::mem::size_of::<u64>() {
        return Vec::new();
    }
    let items = buffer.as_ptr() as *const PDH_FMT_COUNTERVALUE_ITEM_W;
    let mut values = Vec::with_capacity(count as usize);
    for index in 0..count as usize {
        // SAFETY: PDH filled `count` items at this pointer, each within the buffer above; the read
        // is unaligned because the items are laid out inside the byte-sized allocation.
        let item = unsafe { items.add(index).read_unaligned() };
        if item.FmtValue.CStatus == PDH_CSTATUS_VALID_DATA {
            // SAFETY: `PDH_FMT_DOUBLE` was requested, so the union holds the double reading.
            values.push(unsafe { item.FmtValue.Anonymous.doubleValue });
        }
    }
    values
}

/// One collection, then the values of the counter `select` picks out.
fn read(select: fn(&Counters) -> PDH_HCOUNTER) -> Option<Vec<f64>> {
    let mut guard = COUNTERS.lock().ok()?;
    if guard.is_none() {
        *guard = Counters::open();
    }
    let counters = guard.as_ref()?;
    // SAFETY: the query is open and belongs to this module; collecting refreshes its counters.
    let collected = unsafe { PdhCollectQueryData(counters.query) } == 0;
    let values = if collected {
        instance_values(select(counters))
    } else {
        Vec::new()
    };
    if !collected {
        // The query died (a device went away, say): drop it so the next call builds a fresh one.
        *guard = None;
        return None;
    }
    // No instances is how a counter this machine does not publish looks.
    (!values.is_empty()).then_some(values)
}

/// Graphics engine load in percent. Engines overlap — a copy engine idles while the 3D engine
/// works — so the sum is an upper bound and is clamped to one machine's worth of work.
pub(crate) fn gpu_percent() -> Option<f64> {
    let values = read(|handles| handles.gpu)?;
    Some(values.iter().sum::<f64>().clamp(0.0, 100.0).round())
}

/// Package temperature in degrees Celsius: the hottest zone, since the zones name parts of the
/// same silicon.
pub(crate) fn temperature() -> Option<f64> {
    let values = read(|handles| handles.thermal)?;
    // Zones report tenths of a Kelvin; zero means the zone did not answer.
    let hottest = values
        .into_iter()
        .filter(|tenths| *tenths > 0.0)
        .fold(f64::MIN, f64::max);
    (hottest > f64::MIN).then(|| ((hottest / 10.0 - 273.15) * 10.0).round() / 10.0)
}

#[cfg(test)]
mod tests {
    /// 3181 tenths of a Kelvin is 318.1 K, or 45.0 °C.
    #[test]
    fn a_kelvin_reading_becomes_celsius() {
        let tenths: f64 = 3181.0;
        assert_eq!(((tenths / 10.0 - 273.15) * 10.0).round() / 10.0, 45.0);
    }

    #[test]
    fn engines_add_up_but_never_past_a_full_load() {
        let clamp = |values: &[f64]| values.iter().sum::<f64>().clamp(0.0, 100.0).round();
        assert_eq!(clamp(&[12.0, 30.0]), 42.0);
        // Summed engines can exceed the machine's capacity; the island shows a full bar instead.
        assert_eq!(clamp(&[80.0, 90.0]), 100.0);
    }

    /// Prints what this machine publishes, since the answer depends on its drivers and firmware.
    #[test]
    #[ignore = "只读本机的性能计数器，手动运行看这台机器有没有 GPU 与温度"]
    fn reads_this_machines_counters() {
        // The percentage counters need an interval to difference over, hence the pause.
        let _ = super::gpu_percent();
        std::thread::sleep(std::time::Duration::from_millis(600));
        println!("gpu = {:?}", super::gpu_percent());
        println!("temperature = {:?}", super::temperature());
    }
}
