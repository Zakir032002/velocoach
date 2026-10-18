package dev.velocoach.rig

import java.util.concurrent.atomic.AtomicLong
import java.util.concurrent.atomic.AtomicLongArray

object RigStatus {
    @Volatile var servicesAdded = 0
    @Volatile var advertising = false
    @Volatile var central: String? = null
    @Volatile var subscribed: Set<String> = emptySet()
    @Volatile var scenario = "-"
    @Volatile var running = false
    @Volatile var lastError: String? = null
    /** Notifications sent per channel: HRM, CPM, CSC. */
    val sent = AtomicLongArray(3)
    val dropped = AtomicLong()
    val ctrlReceived = AtomicLong()

    fun reset() {
        servicesAdded = 0; advertising = false; central = null; subscribed = emptySet()
        running = false; lastError = null
        for (i in 0 until 3) sent.set(i, 0)
        dropped.set(0); ctrlReceived.set(0)
    }

    fun text(): String = buildString {
        appendLine("services $servicesAdded/4 · advertising $advertising")
        appendLine("central ${central ?: "none"} · subscribed ${subscribed.sorted().joinToString()}")
        appendLine("scenario $scenario · running $running")
        appendLine("sent HRM ${sent[0]} · CPM ${sent[1]} · CSC ${sent[2]} · dropped ${dropped.get()}")
        appendLine("CTRL received ${ctrlReceived.get()}")
        lastError?.let { appendLine("error: $it") }
    }
}
