---
title: "Sinks & shell"
description: "Clipboard, disk, and an arbitrary shell command. Wire up S3 / Slack / scp / whatever you want."
slug: "sinks"
---

# Sinks & shell

Every successful capture fans out to one or more **sinks**. Disk and
clipboard are built in; a shell command is the power-user "send it
anywhere" option.

## Built-in sinks

```toml
[sinks]
clipboard = true   # image lands on the macOS clipboard
disk      = true   # image saved under save_folder
```

Combined with `[general].copy_on_capture`, you have fine-grained
control over which captures hit the clipboard.

## Shell sink

The shell sink runs an arbitrary command after every capture, with the
file path substituted for `$1`:

```toml
[sinks]
shell = "scp $1 user@host:/var/www/img/"
```

Mechanics:

- The command is invoked as `/bin/sh -c "<your command>" -- <path>`,
  so positional `$1` works idiomatically.
- The child is **spawned detached** (`stdin/stdout/stderr` → `/dev/null`)
  so a slow uploader can never stall the capture pipeline.
- Empty / whitespace-only `shell` is a no-op.

## Examples

### Upload to S3 with `rclone`

```toml
[sinks]
shell = "rclone copy $1 s3:my-bucket/screenshots/"
```

### `scp` to a personal web server

```toml
[sinks]
shell = "scp $1 mhorner@home.example.com:/var/www/img/"
```

### Slack via the Slack CLI

```toml
[sinks]
shell = "slack file upload --channels '#design' $1"
```

### A custom uploader returning a URL

Write a tiny shell script that uploads and copies the resulting URL
back to your clipboard:

```sh
#!/usr/bin/env bash
# /usr/local/bin/upload-shot
set -euo pipefail
URL=$(curl -fsSL --upload-file "$1" https://yourservice/upload/)
printf '%s' "$URL" | pbcopy
```

```toml
[sinks]
shell = "/usr/local/bin/upload-shot $1"
```

## Why no built-in cloud uploader?

Building a "share to URL" feature would mean shipping a hosted service
or binding to specific cloud providers. The shell sink is more
flexible *and* keeps the binary entirely network-free in its default
configuration. Open Little Snitch — it'll never light up.
