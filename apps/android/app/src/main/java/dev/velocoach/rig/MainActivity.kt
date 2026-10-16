package dev.velocoach.rig

import android.Manifest
import android.app.Activity
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.view.View
import android.view.WindowManager
import android.widget.ArrayAdapter
import android.widget.Button
import android.widget.LinearLayout
import android.widget.SeekBar
import android.widget.Spinner
import android.widget.Switch
import android.widget.TextView
import dev.velocoach.ffi.encodeLoad
import dev.velocoach.ffi.encodeStart
import dev.velocoach.ffi.encodeStop
import dev.velocoach.ffi.scenarios

class MainActivity : Activity() {
    // POST_NOTIFICATIONS is a runtime permission only from Android 13; asking for it earlier
    // is always "denied", which would keep the rig from starting.
    private val perms = buildList {
        add(Manifest.permission.BLUETOOTH_ADVERTISE)
        add(Manifest.permission.BLUETOOTH_CONNECT)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) add(Manifest.permission.POST_NOTIFICATIONS)
    }.toTypedArray()
    private val ui = Handler(Looper.getMainLooper())
    private lateinit var status: TextView
    private lateinit var scenario: Spinner
    private lateinit var persona: Spinner
    private val scenarioList by lazy { scenarios() }
    private lateinit var statusArea: LinearLayout
    private val humanArea by lazy { buildHumanUi().apply { visibility = View.GONE } }

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
        statusArea = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            listOf(status, scenario, persona, start, stop, off).forEach(::addView)
        }
        setContentView(LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            listOf(humanSwitch, statusArea, humanArea).forEach(::addView)
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
            // In human mode the rig shows no cue information, not even for a frame.
            if (!humanSwitch.isChecked) status.text = RigStatus.text()
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

    private val slider by lazy {
        SeekBar(this).apply {
            max = 2000 // permille of CP: 0 .. 2 x CP
            progress = 700 // 70 % of CP: the pilot scenario's creep reaches the grey zone from here
            minimumHeight = 300
        }
    }
    private val effortLabel by lazy { TextView(this).apply { textSize = 28f; setPadding(32, 32, 32, 32) } }
    private var lastSentMs = 0L
    private val sendLatest = Runnable { sendEffort(slider.progress, SystemClock.elapsedRealtimeNanos()) }

    private fun sendEffort(permille: Int, tNs: Long) {
        lastSentMs = SystemClock.uptimeMillis()
        RigService.driver?.human(permille, tNs)
    }

    private fun buildHumanUi(): View {
        slider.setOnSeekBarChangeListener(object : SeekBar.OnSeekBarChangeListener {
            override fun onProgressChanged(s: SeekBar, p: Int, fromUser: Boolean) {
                if (!fromUser) return
                val tNs = SystemClock.elapsedRealtimeNanos() // the hand moved now
                effortLabel.text = "effort ${p / 10} % of CP"
                ui.removeCallbacks(sendLatest)
                if (SystemClock.uptimeMillis() - lastSentMs >= 50) sendEffort(p, tNs) else ui.postDelayed(sendLatest, 50)
            }

            override fun onStartTrackingTouch(s: SeekBar) {}

            override fun onStopTrackingTouch(s: SeekBar) {
                ui.removeCallbacks(sendLatest)
                sendEffort(s.progress, SystemClock.elapsedRealtimeNanos()) // the final position, always
            }
        })
        return LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            addView(effortLabel)
            addView(slider, LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, 300))
        }
    }

    private val humanSwitch by lazy {
        Switch(this).apply {
            text = "I am the rider"
            setOnCheckedChangeListener { _, on ->
                RigService.driver?.setHumanMode(on)
                if (on) {
                    effortLabel.text = "effort ${slider.progress / 10} % of CP"
                    sendEffort(slider.progress, SystemClock.elapsedRealtimeNanos()) // where the slider already is
                }
                statusArea.visibility = if (on) View.GONE else View.VISIBLE // counters, spinners, buttons
                humanArea.visibility = if (on) View.VISIBLE else View.GONE
            }
        }
    }

    companion object { private const val REQ = 1 }
}
