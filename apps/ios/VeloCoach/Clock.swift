import Darwin

/// The iPhone's ride clock in nanoseconds (mach_continuous_time: counts through sleep).
enum Clock {
    private static let timebase: mach_timebase_info_data_t = {
        var tb = mach_timebase_info_data_t()
        mach_timebase_info(&tb)
        return tb
    }()

    static func nowNs() -> UInt64 {
        ticksToNs(mach_continuous_time())
    }

    static func ticksToNs(_ ticks: UInt64) -> UInt64 {
        ticks * UInt64(timebase.numer) / UInt64(timebase.denom)
    }

    static func nsToTicks(_ ns: UInt64) -> UInt64 {
        ns * UInt64(timebase.denom) / UInt64(timebase.numer)
    }
}
