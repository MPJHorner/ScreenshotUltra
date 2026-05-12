---
title: "Screenshot Ultra: snappy hotkey-first macOS screenshot & recorder"
description: "A native macOS screenshot and screen recorder driven entirely by hotkeys. Region, window, fullscreen, video, GIF — annotate inline with 11 tools, OCR text out of any capture, all local. No cloud, no telemetry, no account."
slug: ""
layout: home
---

<section class="hero">
  <span class="hero-eyebrow"><span class="badge">v{{version}}</span> Native macOS · pure Rust + AppKit</span>
  <h1>Press the key. Drag the box. <span class="accent">It's on your clipboard.</span></h1>
  <p class="lede">Screenshot Ultra is a native menu-bar app for macOS. Press a global hotkey, capture anything on screen — region, window, fullscreen, video, GIF, or clipboard image — annotate inline with eleven tools, and it lands on your clipboard before the shutter sound finishes. All local. No cloud. No telemetry. No account.</p>

  <div class="hero-actions">
    <a class="btn primary" href="{{repo}}/releases/latest" rel="noopener noreferrer">Download for macOS</a>
    <a class="btn ghost" href="{{repo}}" rel="noopener noreferrer">
      <svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><path fill="currentColor" fill-rule="evenodd" d="M8 0C3.58 0 0 3.58 0 8a8 8 0 0 0 5.47 7.59c.4.07.55-.17.55-.38v-1.33c-2.23.48-2.7-1.07-2.7-1.07-.36-.92-.89-1.17-.89-1.17-.73-.5.06-.49.06-.49.81.06 1.23.83 1.23.83.72 1.23 1.88.88 2.34.67.07-.52.28-.88.51-1.08-1.78-.2-3.65-.89-3.65-3.95 0-.87.31-1.59.83-2.15-.08-.21-.36-1.02.08-2.13 0 0 .67-.21 2.2.82a7.6 7.6 0 0 1 4 0c1.53-1.04 2.2-.82 2.2-.82.44 1.11.16 1.92.08 2.13.51.56.82 1.28.82 2.15 0 3.07-1.87 3.75-3.66 3.95.29.25.54.73.54 1.48v2.2c0 .21.15.46.55.38A8 8 0 0 0 16 8c0-4.42-3.58-8-8-8Z"/></svg>
      View on GitHub
    </a>
  </div>

  <div class="hero-oneliner">
    <code>curl -sSL https://raw.githubusercontent.com/MPJHorner/ScreenshotUltra/main/scripts/install.sh | bash</code>
  </div>

  <div class="hero-meta">
    <span><strong>macOS 13+</strong> · universal binary</span>
    <span><strong>Native AppKit</strong>, no Chromium</span>
    <span><strong>MIT</strong> licensed</span>
    <span><strong>0 telemetry</strong> · runs offline</span>
  </div>

  <figure class="hero-figure" style="max-width: 360px; background: transparent; border: 0; box-shadow: none; margin-top: 24px;">
    <img src="{{base}}/img/icon-512.png" alt="Screenshot Ultra app icon — a six-bladed camera aperture iris in shutter red on a macOS squircle" width="360" height="360" />
  </figure>
</section>

<section class="section">
  <div class="section-eyebrow">Why Screenshot Ultra</div>
  <h2>The local Mac alternative to CleanShot, Shottr, and friends.</h2>
  <p class="section-lede">macOS's built-in screenshot tool is great until you need to annotate, pin, repeat, or pipe the result somewhere. The polished competitors solve that — for a yearly subscription and a cloud account you didn't ask for. Screenshot Ultra is the local Mac alternative: every capture lives on your disk in a folder you choose, the annotation editor is a real <code>NSWindow</code> (not a Chromium tab), and the optional "share to anywhere" sink is a shell command <em>you</em> write.</p>

  <div class="feature-grid">
    <div class="feature">
      <h3><span class="feature-icon">⌨</span> Hotkey-first</h3>
      <p>Eleven default global hotkeys. Region, window, fullscreen, clipboard image, repeat-last, pin-last, eyedropper, video, GIF, preferences, cheat sheet. All rebindable.</p>
    </div>
    <div class="feature">
      <h3><span class="feature-icon">✏</span> Native annotation editor</h3>
      <p>A real <code>NSWindow</code> with eleven tools: pen, line, arrow, rect, ellipse, highlighter, redact, counter, text, blur, crop. Five-colour palette, three-step stroke width, full undo / redo.</p>
    </div>
    <div class="feature">
      <h3><span class="feature-icon">📌</span> Pin-to-screen</h3>
      <p>Floating always-on-top windows. Scroll to dim, ⌘+/⌘- to zoom, ⌫ to dismiss. Multiple pins cascade.</p>
    </div>
    <div class="feature">
      <h3><span class="feature-icon">🎬</span> Video + GIF</h3>
      <p>Toggle hotkeys for start/stop recording. Click highlight, microphone, post-stop notifications. GIF post-processed via <code>ffmpeg</code> with a palette filter for crisp small files.</p>
    </div>
    <div class="feature">
      <h3><span class="feature-icon">🔤</span> On-device OCR</h3>
      <p>Apple Vision pulls text out of any capture with one click. Pure on-device, no API keys, no network. Hex on clipboard.</p>
    </div>
    <div class="feature">
      <h3><span class="feature-icon">🔒</span> Local-first</h3>
      <p>No accounts. No analytics. No auto-upload. Captures live on your disk. Optional "share to URL" is a shell command you wire up — every cloud, no SaaS in the middle.</p>
    </div>
  </div>
</section>

<section class="section">
  <div class="section-eyebrow">30-second tour</div>
  <h2>From hotkey to clipboard, every which way.</h2>

  <div class="tour">
    <div class="tour-step">
      <span class="step-num">1 · Region</span>
      <p>Drag a rectangle. Esc cancels. The image is on your clipboard, file in <code>~/Pictures/ScreenshotUltra/</code>, Quick Tray pops up bottom-right.</p>
      <pre>⌃⌥⌘1   →  drag a region</pre>
    </div>
    <div class="tour-step">
      <span class="step-num">2 · Annotate</span>
      <p>Tap the <strong>Edit</strong> button on the Quick Tray. Draw an arrow, pixelate the password, drop a numbered counter on a UI step. ⌘S saves over the original.</p>
      <pre>↵ open editor  →  A pen  →  ⌘S</pre>
    </div>
    <div class="tour-step">
      <span class="step-num">3 · Share</span>
      <p>Want it on S3? Drop into <code>settings.toml</code>:</p>
      <pre>[sinks]
shell = "rclone copy $1 s3:bucket/"</pre>
      <p>Every future capture runs through it. Detached, never blocks.</p>
    </div>
    <div class="tour-step">
      <span class="step-num">4 · Recall</span>
      <p>Press <kbd>⌃⌥⌘/</kbd> for the in-app Cheat Sheet listing every hotkey + editor tool, populated from your live bindings.</p>
      <pre>⌃⌥⌘/  →  cheat sheet
⌃⌥⌘,  →  preferences
⌃⌥⌘P  →  colour picker</pre>
    </div>
  </div>
</section>

<section class="section">
  <div class="cta-card">
    <div>
      <h3>Ready to try it?</h3>
      <p>macOS 13 or newer · ~5 MB universal binary · no installer kernel extensions</p>
    </div>
    <a class="btn primary" href="{{base}}/install/">Install</a>
  </div>
</section>

<section class="section">
  <div class="section-eyebrow">Part of the Ultra family</div>
  <h2>Same posture across four tools.</h2>
  <p class="section-lede">Local-first developer tools. Snappy, native, MIT, no telemetry. Pair Screenshot Ultra with the rest:</p>

  <div class="use-grid">
    <div class="use">
      <h3>MailBox Ultra</h3>
      <p>Local SMTP fake inbox with WebKit HTML preview. Catch every email your dev environment tries to send. <a href="https://github.com/MPJHorner/MailboxUltra" rel="noopener noreferrer">github.com/MPJHorner/MailboxUltra ↗</a></p>
    </div>
    <div class="use">
      <h3>Postbin Ultra</h3>
      <p>Local HTTP request inspector with JSON tree view, forward + replay. <a href="https://github.com/MPJHorner/PostbinUltra" rel="noopener noreferrer">github.com/MPJHorner/PostbinUltra ↗</a></p>
    </div>
    <div class="use">
      <h3>IDE Ultra</h3>
      <p>Local-first native code IDE in pure Rust + egui. <a href="https://github.com/MPJHorner/IdeUltra" rel="noopener noreferrer">github.com/MPJHorner/IdeUltra ↗</a></p>
    </div>
  </div>
</section>
