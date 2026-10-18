# SSMP personal music host in Docker

Docker runs the headless host. The desktop player connects to it under Sources & server. Music remains in your own folder; each user has an authenticated profile. The container needs no graphical desktop or audio device.

Install Docker Engine and Compose on your host. Clone this repository, create a `.env` file beside `compose.yaml`, and set `SSMP_MUSIC_FOLDER` to the absolute path of your music folder. The container mounts that folder read-only. Keep `.env`, database files and media out of Git.

```sh
docker compose build --pull
docker compose up -d
```

The default port is 4534. Restrict it to your LAN or Tailscale network. Add HTTPS with your own reverse proxy when appropriate. The `/api/v1/health` endpoint reports availability without exposing the library. Docker checks it automatically.

Create the first owner with an environment variable. The password is read from the variable, rather than a command argument:

```sh
read -rsp 'SSMP password: ' SSMP_ADMIN_PASSWORD
export SSMP_ADMIN_PASSWORD
docker compose exec -e SSMP_ADMIN_PASSWORD ssmp SSMP --add-user owner --password-env SSMP_ADMIN_PASSWORD --admin
unset SSMP_ADMIN_PASSWORD
```

Create other users using the same command without `--admin`. Connect a desktop player to `http://YOUR_HOST:4534` using that user's credentials. The owner can edit shared metadata; other profiles keep their own playlists, history, lyrics overrides and preferences. Folder watching discovers new files about every five seconds.

## Updates and backups

The Windows in-app updater does not modify containers. Update the source and rebuild the image through the container host:

```sh
git pull --ff-only
docker compose build --pull
docker compose up -d
```

The named `ssmp-data` volume preserves the database, artwork and profiles. Media stays in the external folder. Schedule these steps with your existing deployment tooling if you want automatic container updates. Keep the previous image and a backup of the data volume for rollback; do not remove volumes during an update. Use `SSMP --backup /data/backups/ssmp.sqlite3` inside the container for a consistent SQLite backup.

The runtime uses Debian's FFmpeg package and runs as UID/GID 10001. That user must be able to read the mounted folder. Container compatibility and native desktop platform status are separate checks; consult the accompanying verification report for executed tests.
