package dev.velocoach.rig

import android.os.Handler
import android.os.HandlerThread
import android.os.Process
import android.os.SystemClock
import dev.velocoach.ffi.CtrlReply
import dev.velocoach.ffi.SensorChannel
import dev.velocoach.ffi.Simulator
import dev.velocoach.ffi.VcException
import dev.velocoach.ffi.encodePong

/** Owns the Simulator on its own thread. Only `step`, `onCtrl` and the
 *  Simulator itself run on that thread; everything else just posts to it. */
class SimDriver {
    private val thread = HandlerThread("sim", Process.THREAD_PRIORITY_DISPLAY)
    private lateinit var handler: Handler
    @Volatile private var rig: GattRig? = null
    val queue = NotifyQueue { rig }

    // Touched only on the sim thread.
    private var sim: Simulator? = null

    fun attach(r: GattRig) {
        rig = r
    }

    fun start() {
        thread.start()
        handler = Handler(thread.looper)
        handler.post(loop)
    }

    fun quit() {
        handler.removeCallbacksAndMessages(null)
        handler.post {
            sim?.close()
            sim = null
        }
        thread.quitSafely()
    }

    fun postDelayed(ms: Long, block: () -> Unit) {
        handler.postDelayed(block, ms)
    }

    /** From a CTRL write (binder thread) or the rig's own buttons. */
    fun ctrl(bytes: ByteArray, fromScreen: Boolean) {
        RigStatus.ctrlReceived.incrementAndGet()
        handler.post { onCtrl(bytes, fromScreen) }
    }

    private val loop = object : Runnable {
        override fun run() {
            val next = SystemClock.uptimeMillis() + STEP_MS
            step()
            handler.postAtTime(this, next)
        }
    }

    private fun step() {
        queue.watchdog()
        val s = sim ?: return
        val now = SystemClock.elapsedRealtimeNanos().toULong()
        for (p in s.step(now)) {
            when (p.channel) {
                SensorChannel.HRM -> queue.offer(Uuids.HRM, p.bytes, 0)
                SensorChannel.CPM -> queue.offer(Uuids.CPM, p.bytes, 1)
                SensorChannel.CSC -> queue.offer(Uuids.CSC, p.bytes, 2)
            }
        }
        s.takeDisconnect()?.let { rig?.dropLink(it.toLong()) }
        RigStatus.running = s.isRunning()
    }

    private fun onCtrl(bytes: ByteArray, fromScreen: Boolean) {
        val now = SystemClock.elapsedRealtimeNanos().toULong()
        // The first message is normally LOAD, which replaces the scenario anyway.
        val s = sim ?: Simulator(1u, 0u, null, false).also { sim = it }
        try {
            when (val reply = s.applyCtrl(now, bytes)) {
                is CtrlReply.Ack -> Unit
                is CtrlReply.Pong -> {
                    val tx = SystemClock.elapsedRealtimeNanos().toULong()
                    queue.offer(Uuids.TELEM, encodePong(reply.pingId, reply.tRxNs, tx), -1)
                }
                is CtrlReply.Disconnect -> rig?.dropLink(reply.durMs.toLong())
            }
            RigStatus.scenario = s.name()
            RigStatus.running = s.isRunning()
        } catch (e: VcException) {
            RigStatus.lastError = "CTRL ${if (fromScreen) "(screen)" else "(central)"}: ${e.message}"
        }
    }

    private companion object {
        const val STEP_MS = 50L
    }
}
