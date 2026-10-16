# SSYTDLP with SSMP integration

This workspace copy adds **Settings → SSMP → Add completed downloads to SSMP**. It is off by default. Choose a different inbox only if SSMP uses a custom data folder or the downloader writes to a mounted server share. SSMP's Settings page shows its inbox path.

Only successfully completed files are published. Atomic version-1 JSON manifests include the available title, artist, album, source URL and completed file path. Both SSMP's player and host consume the inbox without racing each other. A failed handoff is reported separately from a successful media download. No SSMP account or running SSMP process is needed to finish a download; the inbox can wait for the player/host.

On Windows, keep `SSYTDLP.exe`, `_internal` and `tools` together. This build uses PyInstaller folder packaging. **Updating only the EXE is insufficient.** Extract into a fresh directory and use that copy. Settings and queues continue to use the existing per-user data folder.

Build on Windows using `source/Build.ps1`, or run `python source/build.py` on the target OS. Native macOS/Linux execution of this update was not tested in this session. Original source licensing stays MIT; bundled dependencies retain their separate licenses.

The containing SSMP workspace has the current VERIFICATION.txt and tested ROLLBACK.sh. The independent downloader ZIP also supplies its pre-SSMP source and executable under `rollback/`, with a rollback script that restores those files and preserves media, tools and user settings.

Antivirus: changing from self-extracting packaging removes that packaging layer; it does not establish that Bitdefender/Malwarebytes will accept the build. Protected retesting is still required. No exclusion or security-setting change is installed by this app.
