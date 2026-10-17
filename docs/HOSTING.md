# Host your collection with SSMP

The desktop player works without hosting or an account. Hosting is optional. All host users share the indexed collection; playlists, favorites, listening history and playback positions belong to each user. Only an owner can change shared metadata. Per-library access restrictions are a later feature.

## On Windows

1. Run `SSMP.exe`, add your media folders and wait for indexing to complete.
2. Open **Sources & server**. Choose an owner username and a password of at least ten characters. Select **Create owner**.
3. For this computer only, retain `127.0.0.1:4534`. To permit LAN/Tailscale connections, enter `0.0.0.0:4534`, then select **Start server**.
4. On another SSMP desktop client, open **Sources & server**, enter `http://HOST_ADDRESS:4534`, and sign in. Use the host's LAN or Tailscale address. A browser is not a player client in version 0.2.

The host is a separate process and continues when the player window closes. **Stop server** stops the process launched by the current window. The log is `host.log` in the data directory. A restart-safe operating-system service is described below; the GUI start button does not install a service.

Closing a client does not delete files. Turning off or sleeping the host makes uncached remote media unavailable. Use **Keep offline** on a track or **Keep offline** to retain completed files on a client, then return to **Local library → Offline** if the host is unavailable. Offline state reconciliation and automatic playlist-download rules are release gates still in progress.

## Build a host without desktop dependencies

Install Rust through rustup and install FFmpeg/ffprobe through your operating system's package manager. From the source folder:

```sh
cargo build --release --no-default-features
./target/release/ssmp-desktop --data "$HOME/.local/share/ssmp" --scan /srv/media
read -r -s SSMP_PASSWORD
export SSMP_PASSWORD
./target/release/ssmp-desktop --data "$HOME/.local/share/ssmp" --add-user owner --password-env SSMP_PASSWORD --admin
unset SSMP_PASSWORD
./target/release/ssmp-desktop --data "$HOME/.local/share/ssmp" --serve --bind 0.0.0.0:4534
```

The binary is named `ssmp-desktop` by Cargo and supports both modes; the headless feature selection excludes egui, libmpv and desktop dialogs. Distributed Windows packages rename it to `SSMP.exe`. Linux/macOS source and CI configuration are supplied; native execution on those systems has not been validated in this Windows session.

To add a regular user, repeat `--add-user NAME --password-env SSMP_PASSWORD` without `--admin`. Credentials are hashed with Argon2id. Login tokens expire after 30 days; signing out through the API revokes the token. Closing a desktop connection only drops the in-memory connection; it is not server-side session revocation.

## systemd example

Use a dedicated service account and choose a data directory it can write. Grant that account read access to the selected media folders. Replace paths and account names to match the actual installation:

```ini
[Unit]
Description=SSMP personal media host
After=network.target

[Service]
User=ssmp
Group=ssmp
ExecStart=/opt/ssmp/ssmp-desktop --data /var/lib/ssmp --serve --bind 0.0.0.0:4534
Restart=on-failure
NoNewPrivileges=true
PrivateTmp=true

[Install]
WantedBy=multi-user.target
```

Create users and scan the library using the same `--data` location. The host initially serves indexed items; ongoing recursive library watching is not implemented. Re-run `--scan /srv/media` after manual changes, or use the SSYTDLP inbox for completed downloads. Both the GUI and host poll the inbox; an atomic claim prevents them from importing the same manifest concurrently.

## Network access

LAN and Tailscale are the initial connection paths. The app never edits router, firewall or VPN settings. Authenticate only over a trusted private network or an HTTPS reverse proxy; this first host exposes plain HTTP itself. A reverse proxy should forward byte-range requests and the Authorization header without caching authenticated media. Only map the configured host port deliberately.

## Backups

Use **Settings → Back up library database**, or run:

```sh
./ssmp-desktop --data /var/lib/ssmp --backup /backup/ssmp-library.sqlite3
```

This creates a consistent SQLite snapshot. Media files and offline media directories need their own backups. Stop the host and player before restoring a database snapshot. Keep an extra copy of the current data directory, restore the snapshot as `library.sqlite3`, and retain the matching media paths. Credentials and sessions are part of the database; protect backups accordingly.

## Diagnostics

- Connection refused: check the host process, bind address, address/port and your network access.
- HTTP 401: verify credentials or sign in again after a token expires.
- HTTP 409: another client changed the profile. Refresh from the host before making another edit; SSMP does not silently overwrite concurrent edits.
- Missing tracks: scan the correct folder on the host and confirm that its service account can read it.
- Playback engine missing: use the complete Windows package; Linux/macOS desktop clients require libmpv installed separately.

## Background folder discovery

Enable **Watch my music folders for changes** on the host desktop, or add `--watch /srv/music` to the headless `--serve` command. The host scans configured local roots on a worker about every five seconds while requests continue. Remote user profiles do not choose host filesystem paths.
