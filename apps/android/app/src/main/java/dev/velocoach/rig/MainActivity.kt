package dev.velocoach.rig

import android.Manifest
import android.app.Activity
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.view.WindowManager
import android.widget.ArrayAdapter
import android.widget.Button
import android.widget.LinearLayout
import android.widget.Spinner
import android.widget.TextView
import dev.velocoach.ffi.encodeLoad
import dev.velocoach.ffi.encodeStart
import dev.velocoach.ffi.encodeStop
import dev.velocoach.ffi.scenarios

class MainActivity : Activity() {
    private val perms = arrayOf(
        Manifest.permission.BLUETOOTH_ADVERTISE,
        Manifest.permission.BLUETOOTH_CONNECT,
        Manifest.permission.POST_NOTIFICATIONS,
    )
    private val ui = Handler(Looper.getMainLooper())
    private lateinit var status: TextView
    private lateinit var scenario: Spinner
    private lateinit var persona: Spinner
    private val scenarioList by lazy { scenarios() }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)

        status = TextView(this).apply { textSize = 14f; setPadding(32, 32, 32, 32) }
        scenario = Spinner(this).apply {
            adapter = ArrayAdapter(context, android.R.layout.simple_spinner_dropdown_item,
                scenarioList.map { "${it.id} ${it.name} (${it.defaultPersona})" })
        }
        persona = Spinner(this).apply {
            adapter = ArrayAdapter(context, android.R.layout.simple_spinner_dropdown_item,
                listOf("scenario default", "dana", "sam", "ned", "calib"))
        }
        val start = Button(this).apply { text = "Load + start ride"; setOnClickListener { loadAndStart() } }
        val stop = Button(this).apply { text = "Stop ride"; setOnClickListener { send(encodeStop()) } }
        val off = Button(this).apply {
            text = "Stop rig"
            setOnClickListener { startService(Intent(context, RigService::class.java).setAction(RigService.ACTION_STOP)) }
        }
        setContentView(LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            listOf(status, scenario, persona, start, stop, off).forEach(::addView)
        })

        if (perms.all { checkSelfPermission(it) == PackageManager.PERMISSION_GRANTED }) startRig()
        else requestPermissions(perms, REQ)
    }

    override fun onRequestPermissionsResult(code: Int, permissions: Array<out String>, results: IntArray) {
        super.onRequestPermissionsResult(code, permissions, results)
        if (code == REQ && results.isNotEmpty() && results.all { it == PackageManager.PERMISSION_GRANTED }) startRig()
        else status.text = "Bluetooth and notification permissions are required."
    }

    override fun onResume() {
        super.onResume()
        ui.post(refresh)
    }

    override fun onPause() {
        ui.removeCallbacks(refresh)
        super.onPause()
    }

    private val refresh = object : Runnable {
        override fun run() {
            status.text = RigStatus.text()
            ui.postDelayed(this, 500)
        }
    }
    /** Only ever called with every permission granted. */
    private fun startRig() = startForegroundService(Intent(this, RigService::class.java))

    private fun loadAndStart() {
        val sc = scenarioList[scenario.selectedItemPosition]
        send(encodeLoad(sc.id, persona.selectedItemPosition.toUByte(), 42u, false))
        send(encodeStart())
    }

    private fun send(ctrl: ByteArray) =
        startForegroundService(Intent(this, RigService::class.java).putExtra(RigService.EXTRA_CTRL, ctrl))

    companion object { private const val REQ = 1 }
}
