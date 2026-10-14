package dev.velocoach.rig

import android.os.SystemClock
import java.util.UUID

/** Exactly one notification in flight. Thread-safe: offered from the sim thread,
 *  released from binder callbacks. */
class NotifyQueue(private val rig: () -> GattRig?) {
    private class Item(val uuid: UUID, val bytes: ByteArray, val channel: Int)

    private val items = ArrayDeque<Item>()
    private var inFlight = false
    private var sentAtMs = 0L

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
            if (r.notify(next.uuid, next.bytes)) {
                inFlight = true
                sentAtMs = SystemClock.elapsedRealtime()
                if (next.channel >= 0) RigStatus.sent.incrementAndGet(next.channel)
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
