package dev.velocoach.rig

import java.util.Locale
import java.util.UUID

object Uuids {
    private fun sig(short: Int): UUID =
        UUID.fromString(String.format(Locale.ROOT, "%08x-0000-1000-8000-00805f9b34fb", short))

    val HRS: UUID = sig(0x180D)
    val HRM: UUID = sig(0x2A37)
    val BODY_LOCATION: UUID = sig(0x2A38)
    val CPS: UUID = sig(0x1818)
    val CPM: UUID = sig(0x2A63)
    val CP_FEATURE: UUID = sig(0x2A65)
    val SENSOR_LOCATION: UUID = sig(0x2A5D)
    val CSCS: UUID = sig(0x1816)
    val CSC: UUID = sig(0x2A5B)
    val CSC_FEATURE: UUID = sig(0x2A5C)
    val CCCD: UUID = sig(0x2902)

    val VCSIM: UUID = UUID.fromString("6c9f0001-5e1a-4c2b-9d3e-7a1b2c3d4e5f")
    val TELEM: UUID = UUID.fromString("6c9f0002-5e1a-4c2b-9d3e-7a1b2c3d4e5f")
    val CTRL: UUID = UUID.fromString("6c9f0003-5e1a-4c2b-9d3e-7a1b2c3d4e5f")
}
