# Finish velocoach: every step, start to end

This guide assumes you have never done any of this before. Follow it top to bottom and do
not skip steps. Each step tells you **what to do**, **what you should see**, and **what to do
if you don't see it**.

**What you need on your desk**
- The MacBook Air (charger plugged in).
- The iPhone 12 and its cable. The MacBook only has USB-C ports, so it is usually a USB-C to
  Lightning cable.
- The iQOO and its USB-C cable. It must be a data cable; some cheap cables only charge.
- Earbuds (optional).
- Your Apple ID password, your GitHub login, and a Google account (for YouTube).

**Total time:** about 5–7 hours. Roughly 2 of those hours are downloads, which you can leave
running while you do other steps.

**New to the project? Read [EXPLAINED.md](EXPLAINED.md) first** (what it is, how it works, with diagrams).

**Want to know what every feature, mode and switch is, and how to show each one?** Read
[WHAT_WE_BUILT.md](WHAT_WE_BUILT.md). This guide is the shortest path; that page lists the extra demo scenes.

**Golden rule:** if anything fails, copy the last 30 lines of the Terminal window and paste
them to Claude in this chat. Don't guess. Every script stops at the first error and prints
the command that failed.

---

## Phase 0 — How to use Terminal (2 min)

1. Press **⌘ Space**, type `Terminal`, and press **Enter**. A window with a prompt ending in
   `%` opens.
2. In this guide, each grey box is a command. Copy the whole box, click in Terminal, press
   **⌘ V**, then press **Enter**.
3. Wait until the `%` prompt comes back before pasting the next box.
4. If Terminal asks for a **Password**, type your Mac login password and press Enter.
   Nothing shows while you type. That is normal.

---

## Phase 1 — Merge the pull request (5 min)

All the code is in pull request #1, and its checks are green. Merge it so everything you do
from here is on `main`.

1. Open https://github.com/Zakir032002/velocoach/pull/1 in your browser and log in.
2. Scroll to the bottom, click the green **Merge pull request**, then **Confirm merge**.
3. **You should see:** a purple "Merged" label.
4. Click the **Actions** tab at the top of the repository. A new **CI** run starts for `main`.
   It needs about 10 minutes to go green, and step 4.3 needs it. Carry on in the meantime.

---

## Phase 2 — Install the tools on the Mac (60–120 min, mostly waiting)

### 2.1 Start the Xcode download FIRST (the longest step)
1. Open the **App Store**, search for **Xcode**, and click **Get** / **Install**. It is about
   15–30 GB.
2. *If the App Store says Xcode needs a newer macOS:* go to  → **System Settings** →
   **General** → **Software Update**, update macOS, then try again.
3. Do 2.2–2.5 while it downloads.

### 2.2 Homebrew (the Mac's installer for developer tools)
```
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
```
Press **Enter** when it asks you to, and type your Mac password. When it finishes, run this
so Terminal can find Homebrew:
```
echo 'eval "$(/opt/homebrew/bin/brew shellenv)"' >> ~/.zprofile && eval "$(/opt/homebrew/bin/brew shellenv)" && brew --version
```
**You should see:** `Homebrew 4.x.x` (or newer).

### 2.3 Git identity and the GitHub command-line tool
```
brew install gh git
git config --global user.name "Zakir Shaik"
git config --global user.email "224626335+Zakir032002@users.noreply.github.com"
```
The email is GitHub's private "noreply" address, so your Gmail address doesn't appear in new
commits.

### 2.4 Log in to GitHub from Terminal
```
gh auth login
```
Answer with the arrow keys and Enter:
1. **GitHub.com**
2. **HTTPS**
3. **Yes** (authenticate Git)
4. **Login with a web browser**

It shows an 8-character code. Press Enter, paste the code in the browser page that opens, and
click **Authorize**. **You should see:** `✓ Logged in as Zakir032002`.

### 2.5 Rust
```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && source "$HOME/.cargo/env" && rustc --version
```
**You should see:** `rustc 1.xx.x`.

### 2.6 Finish Xcode setup (after the download completes)
1. Open **Xcode** once from Launchpad. Accept the licence and let it install its components.
   *If it offers iOS platform support, install it.*
2. Then run:
```
sudo xcode-select -s /Applications/Xcode.app/Contents/Developer && sudo xcodebuild -license accept && xcodebuild -runFirstLaunch && xcodebuild -version
```
**You should see:** `Xcode 2x.x`.

3. In Xcode: **Xcode menu → Settings… → Accounts → "+" → Apple ID**, then sign in.
   **You should see:** your name with **(Personal Team)** under it.

---

## Phase 3 — Get the code and check the Mac (10 min)

```
gh repo clone Zakir032002/velocoach ~/velocoach && cd ~/velocoach && git checkout main && git pull
```
```
cd ~/velocoach && scripts/check-machine.sh
```
**You should see:**
- `ok` for macOS, Homebrew, Xcode, rustup and git.
- `setup` lines for the things the next scripts will install. That's fine.

**If you see `MANUAL`:** that item from Phase 2 is missing. Redo that step.

---

## Phase 4 — The iQOO (the simulated sensors) (15–30 min)

### 4.1 Turn on Developer options (once)
1. On the iQOO, open **Settings → About phone** and tap **Software version** (or **Version**)
   7 times quickly, until it says you are a developer. It may ask for the phone's PIN.
2. In the **Settings** search bar, type `Developer options` and open it.
3. Turn on **USB debugging**.
4. If there is an **Install via USB** switch, turn it on. It may ask you to log in to your
   vivo account.

### 4.2 Connect it
1. Plug the iQOO into the Mac with the data cable and unlock the phone.
2. If the phone asks what the USB connection is for, choose **File transfer**.
3. A popup **"Allow USB debugging?"** appears. Tick **Always allow from this computer** and
   tap **OK**. *If the popup doesn't appear yet,* it will during the next step.

### 4.3 Install the rig app
1. Make sure the **Actions** tab on GitHub shows a **green ✓** for the newest **CI** run on
   `main` (from Phase 1).
2. Then run:
```
cd ~/velocoach && scripts/setup-android.sh --from-ci
```
This installs Java 17 and Android's `adb` tool (no Android Studio), downloads the app CI
built, installs it on the iQOO, gives it its permissions, and opens it. Every install command
is printed before it runs.

**You should see:**
- `OK: VeloCoach Rig is installed and open on …`
- On the phone: the **VeloCoach Rig** app, with a status line `services 4/4 · advertising true`.

**If not:**

| What you see | What to do |
|---|---|
| `no successful CI run on main` | CI on `main` hasn't finished. Wait 10 minutes and re-run the same command. |
| `expected exactly one phone … found 0` | Use another cable. Choose **File transfer** on the phone. Check that USB debugging is on. Re-run. |
| `has not authorised this Mac` | Unlock the phone, tap **Allow** on the popup, and re-run. |
| The phone asks to confirm the install | Tap **Install** / **Allow**, then re-run if the script failed. |

**Fallback, if the script still fails:** build the app on the Mac instead. It takes longer
because it downloads the Android SDK.
```
cd ~/velocoach && scripts/setup-android.sh
```

### 4.4 Battery: don't let OriginOS kill the rig
On the phone, go to **Settings → Battery**, find the **background power consumption**
setting, open **VeloCoach Rig** and choose **Allow** / **No restrictions**. (Use Settings
search if you can't find it.) Turn **Bluetooth on**.

### 4.5 Note the phone's details
```
cat ~/velocoach/build/machine/android-device.txt
```
Copy this output and send it to Claude later (Phase 9). It fills in the iQOO model, Android
and OriginOS versions that are still blank in the docs.

You can unplug the iQOO now. During rides it runs on its own; keep the **VeloCoach Rig**
app open on its screen.

---

## Phase 5 — The iPhone (the coach) (30–60 min)

### 5.1 Build the core and the project
```
cd ~/velocoach && scripts/setup-ios.sh
```
It compiles the Rust core for iPhone (5–10 min the first time), creates the Xcode project and
opens it. **You should see:** `OK: the project is open in Xcode`, and Xcode showing
**VeloCoach**.

### 5.2 Connect the iPhone
1. Plug in the iPhone and unlock it. Tap **Trust** and enter your passcode.
2. In Xcode's top bar, click the device menu (next to **VeloCoach**) and choose **your
   iPhone**.
3. If Xcode says "preparing iPhone for development", wait until it finishes.

### 5.3 Signing (once)
1. In Xcode's left sidebar, click the blue **VeloCoach** project.
2. Choose the **VeloCoach** target, then the **Signing & Capabilities** tab.
3. **Automatically manage signing** should be ticked, and **Team** should be
   **Zakir Shaik (Personal Team)**. If Team is empty or red, pick your Personal Team from
   the list.

### 5.4 Run it
Press **⌘ R** (or the ▶ button).

| What you see | What to do |
|---|---|
| "Developer Mode disabled" | On the iPhone: **Settings → Privacy & Security → Developer Mode → On**. The phone restarts; tap **Turn On**. Press ⌘ R again. |
| "Untrusted Developer" on the iPhone | **Settings → General → VPN & Device Management →** your Apple ID **→ Trust**. Open the app again. |
| Signing error mentioning **UIBackgroundModes** / background modes | Quit Xcode, run `cd ~/velocoach && scripts/setup-ios.sh --no-background`, then press ⌘ R. During rides, keep the app on screen. |
| "bundle identifier is not available" | Send the error to Claude; the fix is a one-line change. |
| Any other red error | Copy it and send it to Claude. |

**You should see:** the VeloCoach app open on the iPhone. When it asks for **Bluetooth**,
tap **Allow**.

### 5.5 Phone settings for rides
- **Settings → Display & Brightness → Auto-Lock → Never** (set it back later).
- Turn on **Do Not Disturb** on both phones, so no notifications appear in recordings.
- Free signing lasts **7 days**. After that the app won't open: plug in and press ⌘ R again.

---

## Phase 6 — First test ride (5 min)

1. Both phones: Bluetooth on, within 1 m of each other. The **VeloCoach Rig** app is open on
   the iQOO.
2. iPhone: tap **Volume check**. **You should hear** a spoken test cue. Connect your earbuds
   if you want to use them.
3. iPhone settings:
   - Scenario **dense**
   - Persona **default**
   - **In-process simulator OFF**
   - **Human rider OFF**
   - **Study mode OFF**
4. Wait for **`link: ready`** at the top. It goes "scanning → connecting → discovering →
   ready" within about 10 s.
5. Tap **Start**. **You should see:**
   - the step counter and quality line moving;
   - at about **0:55** "Surge in five", then **"Go now"** at about **1:00**;
   - a ✓ verdict for it a few seconds later;
   - on the iQOO, `central …` and `running true`.
6. Tap **Stop**.

**If `link` never reaches `ready`:**
- Turn Bluetooth off and on on both phones.
- Close and reopen the rig app.
- Make sure the iQOO isn't connected to anything else.
- Still nothing? Switch **In-process simulator ON**. That runs everything on the iPhone with
  no radio, so the demo still works; its numbers are labelled "no BLE".

---

## Phase 7 — The measured rides (about 40 min)

### 7.1 Latency ride: `dense`, 16 min (fills README rows S1 and S2)
1. Scenario **dense**, everything else as in Phase 6. Tap **Start**.
2. Let it run to the end (16 min). Don't touch the phones.
3. At the end, take a **screenshot** of the latency panel: press the **side button + volume
   up**. It has lines like `T1→T2 p50 … p99 … µs` and `T0→T3 … ms ± … µs`.
4. Tap **Report card** and take a screenshot of that too. You should see the red
   **SIMULATED DATA** banner.

### 7.2 Video ride: `climb_surge`, about 10 min (for the video)
1. Turn on screen recording on the iPhone:
   - Add it once: **Settings → Control Center → add Screen Recording**.
   - Then swipe down from the top-right corner and tap the ⏺ record button.
2. Scenario **climb_surge**, then **Start**. You will hear:
   - "Surge in five" at about **4:55** and "Go now" at **5:00**;
   - the main moment at about **8:50**: a tone, then "Ease off now, your reserve is at
     twenty", then a **✓ complied** badge about 3 s later.
3. At about **9:30**, tap **Stop**, then **Report card**, and scroll it slowly for 5 seconds.
4. Stop the screen recording (tap the red pill at the top). It is saved to **Photos**.
5. *Optional:* record the iQOO's screen at the same time with its own screen recorder
   (quick-settings panel), so you can show the rig side by side.

### 7.3 More scenes for the video (recommended)
The video shows more than one feature. The extra scenes (safety beating everything, "? grade",
the tuner learning) are in [WHAT_WE_BUILT.md § 3](WHAT_WE_BUILT.md#3-the-demo-scenes-one-feature-each).
Three of them are one Terminal command each on the Mac, with no phones needed.

### 7.4 Optional, only if you have time
- **Human mode (E7):**
  - On the iQOO, switch on **I am the rider**; the slider starts at 70 %.
  - On the iPhone, switch on **Human rider (the iQOO slider)**, use scenario
    **creep_pilot**, and press Start.
  - When a cue says to ease off, move the slider down. Your response gets the verdict.
- **Soak (S6):** scenario **soak**, 60 min, done twice. It gives the "no unrecovered failure"
  row. You can skip it for the submission; the README then says *pending*.

---

## Phase 8 — Collect the results (10 min)

1. Plug the iPhone into the Mac. Open **Finder**, click the iPhone in the sidebar, then the
   **Files** tab.
2. Open **VeloCoach** and drag the **captures** folder onto the **Desktop**.
3. Run:
```
cd ~/velocoach && scripts/collect-results.sh --from ~/Desktop/captures
```
**You should see:**
- a summary with `PASS` replay lines (the phone's decisions replay byte for byte);
- `PASS` audit lines;
- and at the end `OK: results/<date-time>` — the folder where everything went.

4. Open the summary:
```
cd ~/velocoach && open "$(ls -d results/* | tail -1)/SUMMARY.md"
```

---

## Phase 9 — Send the numbers to Claude (5 min)

Paste these into this chat:

1. The **SUMMARY.md** text from Phase 8.
2. The **two screenshots** from 7.1 (latency panel and report card).
3. The **android-device.txt** text from 4.5.

Claude will:
- fill in the README status table (S1/S2 → *measured-on-hardware*), `docs/LATENCY.md` and
  the iQOO details;
- open a small pull request for you to merge, the same way as in Phase 1.

Then:
```
cd ~/velocoach && git pull
```

---

## Phase 10 — The 3-minute video (60–90 min)

The script, with the timing and the words to say, is `docs/VIDEO_SCRIPT.md`. Open it on
GitHub or with `open ~/velocoach/docs/VIDEO_SCRIPT.md`. The rules: **3:00 or less**, show
"pending" and "not measured" honestly, and never say "validated", "proven", "users" or that
it works.

1. **Get the clips onto the Mac:**
   - On the iPhone, open **Photos**, select the screen recording, tap **Share → AirDrop →**
     your Mac. It lands in **Downloads**.
   - iQOO recording, if you made one: send it to yourself, for example by Google Drive or
     email.
2. **Record the README table:**
   - On the Mac, open https://github.com/Zakir032002/velocoach and scroll to **Status**.
   - Press **⌘ Shift 5** and choose **Record Entire Screen**.
   - Scroll slowly through the table, then click the stop button in the menu bar.
3. **A desk shot** (0:00–0:15): film both phones and the earbuds on the desk with the Mac's
   camera. Use **QuickTime Player → File → New Movie Recording**. A still photo also works.
4. **Edit in iMovie** (free in the App Store):
   1. **Create New → Movie**, then **Import Media** for all your clips.
   2. Drag them onto the timeline in the script's order.
   3. Click a clip, then the speed icon, to speed up waiting parts (4× or 8×). Don't speed
      up the cue or the ✓.
   4. Add the voiceover: click the **microphone** icon under the viewer and read each
      section of the script.
   5. Add a title card: **Titles** → "velocoach-rs · Zakir Shaik". Add an end card with
      `github.com/Zakir032002/velocoach`.
   6. If the cue played on the speaker, add a small caption "played on the speaker for the
      recording".
   7. Check that the total is 3:00 or less (shown above the timeline).
5. **Export:** **File → Share → File…**, resolution **1080p**, then **Save**.
6. **Upload:**
   - Go to https://youtube.com, sign in, click **Create → Upload video** and pick the file.
   - Set **Visibility: Unlisted** and publish.
   - Copy the link and send it to Claude, who will put it in the README.

---

## Phase 11 — Make it public and submit (20 min)

1. **Run the safety check:**
```
brew install gitleaks && cd ~/velocoach && git pull && scripts/pre-public-check.sh
```
   **You should see:** all `PASS`, then `OK: ready to make public`. If anything says `FAIL`,
   send it to Claude.

2. **Make the repository public:** on GitHub, go to **Settings → General**, scroll to
   **Danger Zone → Change visibility → Make public**, and confirm.

3. **Tag the releases:** **Releases** (right side of the repo page) → **Draft a new
   release**.
   - **v0.1.0:**
     - In **Choose a tag**, type `v0.1.0` and click **Create new tag**. Target: `main`.
     - Title: "v0.1.0 — two phones, real BLE, simulated sensors".
     - Description: paste the v0.1.0 section of `docs/RELEASE_NOTES.md` (Claude fills its
       numbers in Phase 9).
     - Click **Publish release**.
   - **v0.2.0:** the same steps, with `v0.2.0` and the v0.2.0 section. If you skipped the
     soak runs, leave that line as "pending".

4. **Send the submission:**
   - Open `docs/SUBMISSION_MESSAGE.md` and fill in the `[ ]` parts: the repo link, the
     YouTube link and the latency numbers.
   - For the open-source line, add links only to PRs of yours that were **merged**.
   - Send it to Aki the way the trial asked.

**Done.** The demo is public, honest about what is measured, and reproducible.

---

## Quick reference

| I want to… | Command (always start with `cd ~/velocoach &&`) |
|---|---|
| see what the Mac is missing | `scripts/check-machine.sh` |
| reinstall the rig app on the iQOO | `scripts/setup-android.sh --from-ci` |
| rebuild the iPhone app after `git pull` | `scripts/setup-ios.sh`, then ⌘ R in Xcode |
| renew the iPhone app after 7 days | plug in, open Xcode, ⌘ R |
| collect a ride's files | `scripts/collect-results.sh --from ~/Desktop/captures` |
| check before going public | `scripts/pre-public-check.sh` |
| run everything in software (no phones) | `cargo test --workspace` |

The short version of Phases 4–8, for later, is in `docs/PHONE_CHECKLIST.md`.
