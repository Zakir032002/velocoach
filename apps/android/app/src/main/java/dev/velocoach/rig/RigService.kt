package dev.velocoach.rig

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.Service
import android.bluetooth.BluetoothAdapter
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.content.pm.ServiceInfo
import android.os.IBinder
import android.util.Log

/** Owns the GATT server and the simulator thread for as long as the rig runs. */
class RigService : Service() {
    private var rig: GattRig? = null

    private val btState = object : BroadcastReceiver() {
        override fun onReceive(c: Context, i: Intent) {
            if (i.getIntExtra(BluetoothAdapter.EXTRA_STATE, -1) == BluetoothAdapter.STATE_ON) {
                rig?.close()
                RigStatus.servicesAdded = 0
                rig = GattRig(this@RigService, driver ?: return).also { driver?.attach(it); it.open() }
            }
        }
    }

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        if (intent?.action == ACTION_STOP) {
            stopSelf()
            return START_NOT_STICKY
        }
        try {
            // Within 5 s of onStartCommand, and only with the permissions already granted.
            startForeground(NOTIFICATION_ID, notification(), ServiceInfo.FOREGROUND_SERVICE_TYPE_CONNECTED_DEVICE)
        } catch (e: SecurityException) {
            RigStatus.lastError = "startForeground: ${e.message}"
            Log.e(TAG, "startForeground refused", e)
            stopSelf()
            return START_NOT_STICKY
        }
        if (rig == null) {
            val d = SimDriver()
            val r = GattRig(this, d)
            d.attach(r)
            d.start()
            r.open()
            driver = d
            rig = r
            registerReceiver(btState, IntentFilter(BluetoothAdapter.ACTION_STATE_CHANGED))
        }
        // Start/Stop pressed on the rig's own screen: the same path as a CTRL write.
        intent?.getByteArrayExtra(EXTRA_CTRL)?.let { driver?.ctrl(it, fromScreen = true) }
        return START_STICKY
    }

    override fun onDestroy() {
        if (rig != null) unregisterReceiver(btState)
        driver?.quit()
        rig?.close()
        driver = null
        rig = null
        RigStatus.reset()
        super.onDestroy()
    }

    private fun notification(): Notification {
        val nm = getSystemService(NotificationManager::class.java)
        nm.createNotificationChannel(NotificationChannel(CHANNEL, "Sensor rig", NotificationManager.IMPORTANCE_LOW))
        return Notification.Builder(this, CHANNEL)
            .setSmallIcon(android.R.drawable.stat_sys_data_bluetooth)
            .setContentTitle("VeloCoach rig")
            .setContentText("Simulating HR, power and cadence")
            .setOngoing(true)
            .build()
    }

    companion object {
        /** For the human-mode screen: the slider talks to the driver directly. */
        @Volatile var driver: SimDriver? = null
        const val TAG = "vc-rig"
        const val ACTION_STOP = "dev.velocoach.rig.STOP"
        const val EXTRA_CTRL = "ctrl"
        private const val CHANNEL = "rig"
        private const val NOTIFICATION_ID = 1
    }
}
