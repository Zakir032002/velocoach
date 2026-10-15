package dev.velocoach.rig

import android.os.SystemClock
import dev.velocoach.ffi.SensorChannel
import dev.velocoach.ffi.encodeStamp
import java.util.UUID

/** Exactly one notification in flight. Thread-safe: offered from the sim thread,
 *  released from binder callbacks. */
class NotifyQueue(private val rig: () -> GattRig?) {
    private class Item(val uuid: UUID, val bytes: ByteArray, val channel: Int)

    private val items = ArrayDeque<Item>()
    private var inFlight = false
    private var sentAtMs = 0L
    private var stampSeq = 0
    private val sentSeq = IntArray(3)

    /** `channel`: 0 HRM, 1 CPM, 2 CSC, -1 TELEM (for the counters). */
    @Synchronized
    fun offer(uuid: UUID, bytes: ByteArray, channel: Int) {
        if (items.size >= CAPACITY) {
            val sameChar = items.indexOfFirst { it.uuid == uuid }
            items.removeAt(if (sameChar >= 0) sameChar else 0)
            RigStatus.dropped.incrementAndGet()
        }
        items.addLast(Item(uuid, bytes, channel))
        pump()
    }

    @Synchronized
    fun onSent(@Suppress("UNUSED_PARAMETER") status: Int) {
        inFlight = false
        pump()
    }

    /** A new connection: the iPhone restarts its per-channel packet counters too. */
    @Synchronized
    fun onConnected() = resetCounters()

    /** The numbering STAMPs use must restart whenever the iPhone's Engine does: a new
     *  connection, and a new ride (LOAD) on the same connection. */
    @Synchronized
    fun resetCounters() {
        stampSeq = 0
        sentSeq.fill(0)
    }

    @Synchronized
    fun clear() {
        items.clear()
        inFlight = false
    }

    /** Called every 50 ms by the sim thread. */
    @Synchronized
    fun watchdog() {
        if (inFlight && SystemClock.elapsedRealtime() - sentAtMs > WATCHDOG_MS) {
            inFlight = false
            RigStatus.lastError = "onNotificationSent missing for ${WATCHDOG_MS} ms: slot released"
        }
        pump()
    }

    private fun pump() {
        if (inFlight) return
        val r = rig() ?: return
        while (true) {
            val next = items.removeFirstOrNull() ?: return
            if (!r.isSubscribed(next.uuid)) continue
            val t0 = SystemClock.elapsedRealtimeNanos() // T0: handed to the stack now
            if (r.notify(next.uuid, next.bytes)) {
                inFlight = true
                sentAtMs = SystemClock.elapsedRealtime()
                if (next.channel >= 0) {
                    RigStatus.sent.incrementAndGet(next.channel)
                    sentSeq[next.channel] = (sentSeq[next.channel] + 1) and 0xFFFF
                    stampSeq = (stampSeq + 1) and 0xFFFF
                    val chan = arrayOf(SensorChannel.HRM, SensorChannel.CPM, SensorChannel.CSC)[next.channel]
                    items.addLast(Item(Uuids.TELEM, encodeStamp(stampSeq.toUShort(), chan, sentSeq[next.channel].toUShort(), t0.toULong()), -1))
                }
            } else {
                items.addFirst(next) // the stack is busy: retry on the next offer or watchdog tick
            }
            return
        }
    }

    private companion object {
        const val CAPACITY = 32
        const val WATCHDOG_MS = 500L
    }
}
