//! x86_64 Xen Time

use {crate::SHARED_INFO, core::arch::x86_64::_rdtsc};

/// Gets the current system time represented as the number of nanoseconds since 1970-01-01 00:00:00 UTC
///
/// Using 64 bit nanosecond timestamps will break on July 21st 2554.
pub fn get_system_time() -> u64 {
    let shared_info = &unsafe { *SHARED_INFO };

    let mut wc_version;
    let mut version;

    let mut seconds;
    let mut nanoseconds;
    let mut system_time;
    let mut old_tsc;

    let mut shift;
    let mut mul;

    loop {
        // if the lowest bit of either version is 1 then the time is being updated so spin until finished
        loop {
            wc_version = shared_info.wc_version;
            version = shared_info.vcpu_info[0].time.version;
            if !(version & 1 == 1 || wc_version & 1 == 1) {
                break;
            }
        }

        seconds = shared_info.wc_sec;
        nanoseconds = shared_info.wc_nsec;
        system_time = shared_info.vcpu_info[0].time.system_time;
        old_tsc = shared_info.vcpu_info[0].time.tsc_timestamp;

        shift = shared_info.vcpu_info[0].time.tsc_shift;
        mul = shared_info.vcpu_info[0].time.tsc_to_system_mul;

        // break only if all values were read from the same update version
        if !(version != shared_info.vcpu_info[0].time.version
            || wc_version != shared_info.wc_version)
        {
            break;
        }
    }

    // convert TSC to nanoseconds
    let tsc_nanos = {
        let mut delta = unsafe { _rdtsc() } - old_tsc;

        if shift < 0 {
            delta >>= -shift;
        } else {
            delta <<= shift;
        }

        (delta * u64::from(mul)) >> 32
    };

    system_time // time in nanoseconds since boot
        + tsc_nanos // Time Stamp Counter nanoseconds
        + (u64::from(seconds) * 1_000_000_000) // current system time seconds
        + u64::from(nanoseconds) // current system time nanoseconds
}
