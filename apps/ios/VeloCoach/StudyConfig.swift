/// Fixed by STUDY.md. The hash is that file's commit (M1); every record carries it.
enum StudyConfig {
    /// Set to `git log -1 --format=%h -- STUDY.md` after the final pre-registration
    /// commit; scripts/check-protocol-hash.sh refuses to pass until it matches.
    static let protocolHash = "92aa968"
    static let riders = ["R00", "R01", "R02", "R03"] // R00: the uncounted dry run
    static func seed(rider: Int, session: Int) -> UInt64 { UInt64(1_000 * rider + session) }
}
