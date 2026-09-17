# EagleCast TODO

EagleCast is a Windows-first Rust application for the Polycom EagleEye PTZ camera.

Primary goals:

- ingest the EagleEye's low-latency network video stream
- provide immediate live camera-position feedback
- control PTZ through the Raspberry Pi REST API
- correct the camera's physical roll / crooked horizon in real time
- automatically crop and scale transformed video
- expose the corrected result as a Windows virtual camera
- optionally record the corrected video locally

The physical camera is mounted at an angle, so horizon correction is a core feature rather than an optional effect.

---

# Development rules

- [x] Rust project
- [x] Windows-first
- [x] Development branch is `main`
- [x] Keep the project split into sensible modules from the beginning
- [x] Do not allow the application to become one giant `main.rs`
- [x] `TODO.md` must be updated on EVERY code/configuration edit
- [x] Development instructions must use exact FIND -> REPLACE edits
- [x] FIND blocks must come from the current GitHub source
- [x] Preserve indentation exactly
- [x] Prefer meaningful development slices instead of tiny one-line edits
- [x] Every development slice is committed and pushed
- [x] Broken/intermediate states may also be committed when necessary so GitHub remains the source of truth
- [x] Use `git add .`
- [x] Do not rely on `git status`, `git log`, or similar inspection commands as part of the normal workflow
- [x] Format/build/Clippy/test gates should be run as appropriate before the next slice
- [x] The first real commit must already contain a useful working proof of concept

---

# Known hardware / network

## EagleEye video stream

Known-good FFplay command:

    F:\ffmpeg-master-latest-win64-gpl\bin\ffplay.exe
        -fflags nobuffer
        -flags low_delay
        -framedrop
        -analyzeduration 0
        -probesize 32
        -sync ext
        -window_title EagleEye
        "udp://@239.42.0.1:5000?fifo_size=1000000&overrun_nonfatal=1"

Multicast stream:

    udp://@239.42.0.1:5000

Important characteristics:

- extremely low latency is preferred
- dropping stale frames is preferable to accumulating latency
- audio is currently irrelevant
- multicast reception already works on the Windows development machine

---

# Raspberry Pi PTZ API

LAN base URL:

    http://192.168.1.201:8765

WireGuard base URL:

    http://10.10.10.11:8765

The API currently has no authentication and must remain on the trusted LAN/VPN.

## Camera state

    GET /api/state

Expected fields include:

    pan_degrees
    tilt_degrees
    zoom_percent

Expected ranges:

    pan:  -170 .. 170 degrees
    tilt:  -30 .. 90 degrees
    zoom:     0 .. 100 percent

## Absolute PTZ

    POST /api/set

Example:

    {
        "pan_degrees": -70,
        "tilt_degrees": 28,
        "zoom_percent": 55
    }

## Relative PTZ

    POST /api/nudge

Example:

    {
        "pan_delta_degrees": 5,
        "tilt_delta_degrees": 0,
        "zoom_delta_percent": 0
    }

Zoom-aware fine adjustment:

    {
        "pan_delta_degrees": 1,
        "tilt_delta_degrees": -1,
        "zoom_aware": true
    }

## Home

    POST /api/home

## Health / information

    GET /api/health
    GET /api/info

## Snapshot

    GET /snapshot.jpg

## Presets

    GET  /api/presets

    POST /api/presets/save
    {
        "name": "desk",
        "description": "Desk view"
    }

    POST /api/presets/goto
    {
        "name": "desk"
    }

    POST /api/presets/delete
    {
        "name": "desk"
    }

## MCP endpoint

Not required by the normal EagleCast application but available for future experimentation:

    http://192.168.1.201:8765/mcp/
    http://10.10.10.11:8765/mcp/

---

# Phase 1 - Working video proof of concept

Goal: the first useful EagleCast build already displays the camera and fixes the crooked image.

## Current development slice - POC A

The first implementation slice is now targeting the complete basic live path rather than a framework-only skeleton:

- [ ] Verify EagleCast's eframe 0.36 root-UI shell builds and runs on Windows
- [ ] Verify the Windows mixed-DPI manifest behaves correctly across displays/scaling levels
- [ ] Verify FFmpeg launches from the known local installation
- [ ] Verify multicast EagleEye video appears in EagleCast
- [ ] Verify stale decoded frames are replaced rather than queued
- [ ] Verify live manual roll correction
- [ ] Verify automatic crop compensation during roll correction
- [ ] Verify `/api/state` polling against the Raspberry Pi
- [ ] Verify pan/tilt/zoom feedback follows movement made with the physical remote

Roadmap items below remain unchecked until the implementation has been tested against the real camera.

- [ ] Create basic Windows GUI
- [ ] Connect to the EagleEye multicast stream
- [ ] Decode incoming video
- [ ] Display live video inside the EagleCast window
- [ ] Keep latency aggressively low
- [ ] Drop stale frames instead of building a frame queue
- [ ] Show stream-connected/disconnected state
- [ ] Show incoming resolution
- [ ] Show measured input FPS
- [ ] Show rendered/output FPS
- [ ] Add manually adjustable roll correction
- [ ] Apply roll correction live
- [ ] Automatically crop invalid image corners created by rotation
- [ ] Scale cropped image back into the preview area
- [ ] Preserve the correct aspect ratio
- [ ] Allow roll correction with decimal-degree precision
- [ ] Store roll setting between launches
- [ ] Handle temporary multicast loss without crashing
- [ ] Automatically reconnect when the stream returns

Initial success criterion:

    EagleEye UDP multicast
        ->
    EagleCast
        ->
    decoded video
        ->
    corrected horizon
        ->
    live preview

No virtual camera is required yet for Phase 1.

---

# Phase 2 - Raspberry Pi camera state

Goal: make EagleCast extremely aware of where the physical camera currently is.

- [ ] Add Raspberry Pi REST client
- [ ] Make REST base URL configurable
- [ ] Default LAN address to `http://192.168.1.201:8765`
- [ ] Allow easy switch to WireGuard address `http://10.10.10.11:8765`
- [ ] Check `/api/health`
- [ ] Read `/api/info`
- [ ] Poll `/api/state`
- [ ] Display live pan position
- [ ] Display live tilt position
- [ ] Display live zoom percentage
- [ ] Update PTZ feedback quickly enough to visibly follow remote-control movement
- [ ] Avoid blocking the video/render thread with HTTP requests
- [ ] Show API connected/disconnected state
- [ ] Detect stale PTZ state
- [ ] Recover automatically if the Pi temporarily disappears
- [ ] Track last successful state update timestamp

Desired UI feedback:

    CAMERA       CONNECTED
    Pan          -70.0 deg
    Tilt          28.0 deg
    Zoom          55 %
    Preset        desk
    State age     42 ms

Camera position feedback is more important than software PTZ control because most normal movement will be performed with the physical remote and camera presets.

---

# Phase 3 - PTZ controls

- [ ] Absolute pan control
- [ ] Absolute tilt control
- [ ] Absolute zoom control
- [ ] Relative pan nudges
- [ ] Relative tilt nudges
- [ ] Relative zoom nudges
- [ ] Fine 1-degree nudge controls
- [ ] Support `zoom_aware`
- [ ] Home button
- [ ] Disable unsafe/out-of-range values in the UI
- [ ] Clamp values to the API's known supported ranges
- [ ] Clearly distinguish requested PTZ state from confirmed PTZ state
- [ ] Do not assume movement completed until `/api/state` confirms it

---

# Phase 4 - Presets

- [ ] Fetch preset list from `/api/presets`
- [ ] Display available presets
- [ ] Go to preset
- [ ] Save current position as preset
- [ ] Add preset description
- [ ] Delete preset with confirmation
- [ ] Show active/nearest preset when identifiable
- [ ] Refresh PTZ state continuously while a preset move is happening
- [ ] Make preset buttons large/easy to hit

Likely commonly used presets:

    desk
    workbench
    room
    closeup

These names are examples only and must not be hard-coded as required presets.

---

# Phase 5 - Position-dependent horizon correction

Goal: automatically correct the horizon based on physical camera position.

A single fixed roll value may not be sufficient if the apparent roll changes as the PTZ head moves.

- [ ] Associate stabilization information with PTZ position
- [ ] Allow roll calibration for individual presets
- [ ] Automatically apply the calibrated roll when entering a preset
- [ ] Investigate whether apparent roll varies smoothly with pan/tilt
- [ ] Support calibration points outside named presets
- [ ] Interpolate roll correction between calibration points
- [ ] Avoid visible correction jumps during PTZ movement
- [ ] Smooth automatic roll changes
- [ ] Allow automatic correction to be overridden manually
- [ ] Show current calculated roll correction
- [ ] Show whether correction is MANUAL / PRESET / INTERPOLATED

Possible model:

    pan + tilt + zoom
        ->
    calibration map
        ->
    desired roll
        ->
    smoothing
        ->
    frame transform

Do not over-engineer interpolation until real camera measurements show that it is useful.

---

# Phase 6 - Video transform quality

- [ ] Determine largest valid crop after arbitrary rotation
- [ ] Minimize unnecessary digital zoom
- [ ] Provide optional fixed output crop
- [ ] Support 16:9 output
- [ ] Support 1920x1080 output
- [ ] Investigate 2560x1440 output if useful
- [ ] Preserve smooth frame pacing
- [ ] Avoid repeated CPU-side frame copies where practical
- [ ] Investigate GPU accelerated transform/rendering
- [ ] Measure transform latency
- [ ] Measure full input-to-preview latency
- [ ] Expose latency diagnostics in a debug panel
- [ ] Optional horizontal mirror
- [ ] Optional vertical flip
- [ ] Optional 90/180/270-degree rotation for future mounting configurations

---

# Phase 7 - Windows virtual camera

Goal: expose EagleCast's corrected output as a normal Windows camera.

Target:

    Applications
        ->
    Camera selector
        ->
    EagleCast Virtual Camera

- [ ] Research final Media Foundation architecture
- [ ] Create Media Foundation virtual-camera prototype
- [ ] Register virtual camera on Windows
- [ ] Feed processed EagleCast frames into the virtual camera
- [ ] Advertise sensible supported formats
- [ ] Support 1920x1080
- [ ] Support at least 30 FPS
- [ ] Investigate 60 FPS
- [ ] Verify operation in Windows camera applications
- [ ] Verify operation in browser WebRTC camera selection
- [ ] Verify operation in Teams/Meet/Discord-style applications
- [ ] Verify operation in OBS
- [ ] Handle consumer application opening/closing the camera cleanly
- [ ] Avoid leaving dead virtual-camera registrations after crashes
- [ ] Add install/register command if Windows requires one
- [ ] Add uninstall/unregister command
- [ ] Name virtual device clearly

Preferred name:

    EagleCast Virtual Camera

---

# Phase 8 - Recording

- [ ] Record corrected output directly from EagleCast
- [ ] Start/stop recording button
- [ ] Show recording duration
- [ ] Show destination file
- [ ] Configurable recording directory
- [ ] Timestamp filenames
- [ ] Choose sensible default codec
- [ ] Hardware encoding where available
- [ ] Avoid recording causing preview latency
- [ ] Gracefully finalize recordings after interruption
- [ ] Snapshot corrected frame
- [ ] Snapshot raw frame for comparison/debugging

---

# Phase 9 - Configuration

Persist at least:

- [ ] REST API address
- [ ] stream URL
- [ ] output resolution
- [ ] desired output FPS
- [ ] manual roll correction
- [ ] automatic crop setting
- [ ] PTZ polling interval
- [ ] last selected preset
- [ ] calibration data
- [ ] recording directory
- [ ] virtual-camera settings
- [ ] window size/position where useful

Configuration should be human-readable where practical.

---

# Phase 10 - UI

Target layout:

    +------------------------------------------------------+--------------------+
    |                                                      | CAMERA             |
    |                                                      | API       CONNECTED|
    |                                                      | Pan       -70.0 deg|
    |                                                      | Tilt       28.0 deg|
    |                 VIDEO PREVIEW                        | Zoom       55 %    |
    |                                                      | Preset     desk    |
    |                                                      |                    |
    |                                                      | STABILIZATION      |
    |                                                      | Roll      -13.8 deg|
    |                                                      | Mode      AUTO     |
    |                                                      |                    |
    +------------------------------------------------------+--------------------+
    | STREAM  LIVE | 30.0 FPS | PTZ 42 ms | VCAM ACTIVE | REC 00:12:41        |
    +---------------------------------------------------------------------------+

- [ ] Large video preview
- [ ] Avoid wasting significant space on decorations
- [ ] Clearly visible LIVE/OFFLINE state
- [ ] Clearly visible Pi/API state
- [ ] PTZ values visible without opening another panel
- [ ] Roll correction visible at all times
- [ ] Presets directly accessible
- [ ] Virtual-camera state visible
- [ ] Recording state visible
- [ ] Diagnostics available without cluttering normal operation
- [ ] Support dark UI
- [ ] UI must remain responsive during stream/API failures

---

# Phase 11 - Diagnostics / robustness

- [ ] Structured logging
- [ ] Useful startup diagnostics
- [ ] Report multicast bind errors clearly
- [ ] Report decode errors clearly
- [ ] Report Pi/API errors clearly
- [ ] Do not flood logs for repeated identical transient errors
- [ ] Measure dropped frames
- [ ] Measure decode FPS
- [ ] Measure render FPS
- [ ] Measure PTZ state update interval
- [ ] Detect stream stalls
- [ ] Automatic stream recovery
- [ ] Automatic REST recovery
- [ ] Clean shutdown
- [ ] Clean worker-thread/task shutdown
- [ ] Handle Pi reboot while EagleCast remains open
- [ ] Handle network adapter interruption
- [ ] Handle EagleEye video disappearing and returning

---

# Phase 12 - Packaging

- [ ] Windows release build
- [ ] Application icon
- [ ] Version information
- [ ] README
- [ ] Basic usage instructions
- [ ] Explain multicast/network requirements
- [ ] Explain Raspberry Pi requirement
- [ ] Explain trusted-LAN/VPN security assumption
- [ ] Installer or portable release decision
- [ ] Virtual-camera installation documented
- [ ] Produce first GitHub release

---

# Later / experimental ideas

- [ ] Visual horizon calibration tool
- [ ] Click two points defining a known-horizontal line and calculate roll automatically
- [ ] Automatic horizon detection
- [ ] Automatic face-aware framing
- [ ] Optional subject tracking
- [ ] Smooth PTZ movement generated by EagleCast
- [ ] Keyboard PTZ shortcuts
- [ ] Gamepad PTZ input
- [ ] Stream Deck integration
- [ ] MIDI/controller integration
- [ ] Multiple calibration profiles
- [ ] Multiple EagleEye cameras
- [ ] Browser/MCP control integration
- [ ] Picture-in-picture debug view showing raw vs corrected frames
- [ ] PTZ motion history/graph
- [ ] Frame timestamp overlay for latency testing
- [ ] External API so other local programs can control EagleCast