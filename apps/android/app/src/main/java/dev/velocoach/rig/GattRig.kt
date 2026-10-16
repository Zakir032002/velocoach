package dev.velocoach.rig

import android.annotation.SuppressLint
import android.bluetooth.BluetoothDevice
import android.bluetooth.BluetoothGatt
import android.bluetooth.BluetoothGattCharacteristic
import android.bluetooth.BluetoothGattCharacteristic.PERMISSION_READ
import android.bluetooth.BluetoothGattCharacteristic.PERMISSION_WRITE
import android.bluetooth.BluetoothGattCharacteristic.PROPERTY_NOTIFY
import android.bluetooth.BluetoothGattCharacteristic.PROPERTY_READ
import android.bluetooth.BluetoothGattCharacteristic.PROPERTY_WRITE
import android.bluetooth.BluetoothGattCharacteristic.PROPERTY_WRITE_NO_RESPONSE
import android.bluetooth.BluetoothGattDescriptor
import android.bluetooth.BluetoothGattServer
import android.bluetooth.BluetoothGattServerCallback
import android.bluetooth.BluetoothGattService
import android.bluetooth.BluetoothManager
import android.bluetooth.BluetoothProfile
import android.bluetooth.BluetoothStatusCodes
import android.bluetooth.le.AdvertiseCallback
import android.bluetooth.le.AdvertiseData
import android.bluetooth.le.AdvertiseSettings
import android.content.Context
import android.os.Build
import android.os.ParcelUuid
import android.os.SystemClock
import android.util.Log
import java.util.UUID
import java.util.concurrent.ConcurrentHashMap

/** The rig's GATT server. Every method is safe to call from any thread.
 *  MissingPermission is suppressed because RigService starts only after the
 *  activity has been granted BLUETOOTH_ADVERTISE and BLUETOOTH_CONNECT. */
@SuppressLint("MissingPermission")
class GattRig(private val ctx: Context, private val driver: SimDriver) {
    private val manager = ctx.getSystemService(BluetoothManager::class.java)
    private var server: BluetoothGattServer? = null
    private val pending = ArrayDeque<BluetoothGattService>()
    private val chars = ConcurrentHashMap<UUID, BluetoothGattCharacteristic>()
    private val readValues = ConcurrentHashMap<UUID, ByteArray>()
    private val subscribed: MutableSet<UUID> = ConcurrentHashMap.newKeySet()
    @Volatile private var central: BluetoothDevice? = null
    @Volatile private var advertising = false

    fun open() {
        val s = manager.openGattServer(ctx, callback)
        if (s == null) {
            RigStatus.lastError = "openGattServer returned null (is Bluetooth on?)"
            return
        }
        server = s
        synchronized(pending) { pending.addAll(listOf(hrs(), cps(), cscs(), vcsim())) }
        addNext()
    }

    fun close() {
        stopAdvertising()
        server?.close()
        server = null
    }

    fun isSubscribed(uuid: UUID): Boolean = central != null && uuid in subscribed

    /** One notification; NotifyQueue makes sure only one is in flight. */
    @Suppress("DEPRECATION")
    fun notify(uuid: UUID, bytes: ByteArray): Boolean {
        val dev = central ?: return false
        val ch = chars[uuid] ?: return false
        val s = server ?: return false
        return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            s.notifyCharacteristicChanged(dev, ch, false, bytes) == BluetoothStatusCodes.SUCCESS
        } else {
            ch.value = bytes // Android 12: the pre-33 API
            s.notifyCharacteristicChanged(dev, ch, false)
        }
    }

    /** The DISCONNECT fault: drop the link, stay silent for `ms`, then advertise again. */
    fun dropLink(ms: Long) {
        stopAdvertising()
        central?.let { server?.cancelConnection(it) }
        driver.postDelayed(ms) { startAdvertising() }
    }

    // ---- the service table --------------------------------------------------------

    private fun notifyChar(uuid: UUID) =
        BluetoothGattCharacteristic(uuid, PROPERTY_NOTIFY, 0).apply {
            addDescriptor(BluetoothGattDescriptor(Uuids.CCCD, BluetoothGattDescriptor.PERMISSION_READ or BluetoothGattDescriptor.PERMISSION_WRITE))
            chars[uuid] = this
        }

    private fun readChar(uuid: UUID, value: ByteArray) =
        BluetoothGattCharacteristic(uuid, PROPERTY_READ, PERMISSION_READ).also { readValues[uuid] = value }

    private fun service(uuid: UUID, vararg cs: BluetoothGattCharacteristic) =
        BluetoothGattService(uuid, BluetoothGattService.SERVICE_TYPE_PRIMARY).apply { cs.forEach { addCharacteristic(it) } }

    private fun hrs() = service(Uuids.HRS, notifyChar(Uuids.HRM), readChar(Uuids.BODY_LOCATION, byteArrayOf(1)))

    private fun cps() = service(
        Uuids.CPS,
        notifyChar(Uuids.CPM),
        readChar(Uuids.CP_FEATURE, byteArrayOf(0x08, 0, 0, 0)),
        readChar(Uuids.SENSOR_LOCATION, byteArrayOf(6)),
    )

    private fun cscs() = service(Uuids.CSCS, notifyChar(Uuids.CSC), readChar(Uuids.CSC_FEATURE, byteArrayOf(0x02, 0)))

    private fun vcsim() = service(
        Uuids.VCSIM,
        notifyChar(Uuids.TELEM),
        BluetoothGattCharacteristic(Uuids.CTRL, PROPERTY_WRITE or PROPERTY_WRITE_NO_RESPONSE, PERMISSION_WRITE),
    )

    /** One addService at a time: the next is added from onServiceAdded. */
    private fun addNext() {
        val next = synchronized(pending) { pending.removeFirstOrNull() }
        if (next == null) startAdvertising() else server?.addService(next)
    }

    // ---- advertising ----------------------------------------------------------------

    private val advertiseCallback = object : AdvertiseCallback() {
        override fun onStartSuccess(settingsInEffect: AdvertiseSettings) {
            advertising = true
            RigStatus.advertising = true
        }

        override fun onStartFailure(errorCode: Int) {
            advertising = false
            RigStatus.advertising = false
            RigStatus.lastError = "advertising failed: $errorCode"
        }
    }

    private fun startAdvertising() {
        if (advertising || central != null) return
        val adv = manager.adapter?.bluetoothLeAdvertiser ?: run {
            RigStatus.lastError = "no LE advertiser"
            return
        }
        val settings = AdvertiseSettings.Builder()
            .setAdvertiseMode(AdvertiseSettings.ADVERTISE_MODE_LOW_LATENCY)
            .setTxPowerLevel(AdvertiseSettings.ADVERTISE_TX_POWER_MEDIUM)
            .setConnectable(true)
            .setTimeout(0)
            .build()
        val data = AdvertiseData.Builder()
            .setIncludeDeviceName(false)
            .addServiceUuid(ParcelUuid(Uuids.CPS))
            .addServiceUuid(ParcelUuid(Uuids.HRS))
            .addServiceUuid(ParcelUuid(Uuids.CSCS))
            .build()
        val scanResponse = AdvertiseData.Builder()
            .setIncludeDeviceName(false)
            .addServiceUuid(ParcelUuid(Uuids.VCSIM))
            .build()
        adv.startAdvertising(settings, data, scanResponse, advertiseCallback)
    }

    private fun stopAdvertising() {
        manager.adapter?.bluetoothLeAdvertiser?.stopAdvertising(advertiseCallback)
        advertising = false
        RigStatus.advertising = false
    }

    // ---- callbacks (binder threads) --------------------------------------------------

    private val callback = object : BluetoothGattServerCallback() {
        override fun onServiceAdded(status: Int, service: BluetoothGattService) {
            if (status != BluetoothGatt.GATT_SUCCESS) {
                RigStatus.lastError = "addService ${service.uuid} failed: $status"
                return
            }
            RigStatus.servicesAdded += 1
            addNext()
        }

        override fun onConnectionStateChange(device: BluetoothDevice, status: Int, newState: Int) {
            when (newState) {
                BluetoothProfile.STATE_CONNECTED -> {
                    central = device
                    RigStatus.central = device.address
                    driver.queue.onConnected() // both phones restart their packet numbering
                    stopAdvertising() // one central at a time
                }
                BluetoothProfile.STATE_DISCONNECTED -> if (device == central) {
                    central = null
                    subscribed.clear()
                    RigStatus.central = null
                    RigStatus.subscribed = emptySet()
                    driver.queue.clear()
                    startAdvertising()
                }
            }
        }

        override fun onCharacteristicReadRequest(device: BluetoothDevice, requestId: Int, offset: Int, ch: BluetoothGattCharacteristic) {
            val v = readValues[ch.uuid]
            if (v == null || offset > v.size) {
                server?.sendResponse(device, requestId, BluetoothGatt.GATT_READ_NOT_PERMITTED, offset, null)
            } else {
                server?.sendResponse(device, requestId, BluetoothGatt.GATT_SUCCESS, offset, v.copyOfRange(offset, v.size))
            }
        }

        override fun onDescriptorReadRequest(device: BluetoothDevice, requestId: Int, offset: Int, d: BluetoothGattDescriptor) {
            val on = d.characteristic.uuid in subscribed
            val v = if (on) BluetoothGattDescriptor.ENABLE_NOTIFICATION_VALUE else BluetoothGattDescriptor.DISABLE_NOTIFICATION_VALUE
            server?.sendResponse(device, requestId, BluetoothGatt.GATT_SUCCESS, 0, v)
        }

        override fun onDescriptorWriteRequest(
            device: BluetoothDevice, requestId: Int, d: BluetoothGattDescriptor,
            preparedWrite: Boolean, responseNeeded: Boolean, offset: Int, value: ByteArray,
        ) {
            if (d.uuid == Uuids.CCCD) {
                val uuid = d.characteristic.uuid
                if (value.contentEquals(BluetoothGattDescriptor.ENABLE_NOTIFICATION_VALUE)) subscribed.add(uuid) else subscribed.remove(uuid)
                RigStatus.subscribed = subscribed.map { shortName(it) }.toSet()
            }
            if (responseNeeded) server?.sendResponse(device, requestId, BluetoothGatt.GATT_SUCCESS, 0, null)
        }

        override fun onCharacteristicWriteRequest(
            device: BluetoothDevice, requestId: Int, ch: BluetoothGattCharacteristic,
            preparedWrite: Boolean, responseNeeded: Boolean, offset: Int, value: ByteArray,
        ) {
            val rxNs = SystemClock.elapsedRealtimeNanos() // b: the moment the write arrived
            // Answer first: never make the central wait on the simulator.
            if (responseNeeded) server?.sendResponse(device, requestId, BluetoothGatt.GATT_SUCCESS, 0, null)
            if (ch.uuid == Uuids.CTRL) driver.ctrl(value.copyOf(), fromScreen = false, rxNs = rxNs)
        }

        override fun onNotificationSent(device: BluetoothDevice, status: Int) {
            driver.queue.onSent(status)
        }

        override fun onMtuChanged(device: BluetoothDevice, mtu: Int) {
            Log.i(RigService.TAG, "MTU $mtu") // every packet is at most 20 bytes: fits the default 23
        }
    }

    private fun shortName(u: UUID) = when (u) {
        Uuids.HRM -> "HRM"
        Uuids.CPM -> "CPM"
        Uuids.CSC -> "CSC"
        Uuids.TELEM -> "TELEM"
        else -> u.toString().take(8)
    }
}
