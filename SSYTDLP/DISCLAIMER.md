# Personal use and source transparency

SSYTDLP is intended for personal use. Download only material you own or have
permission to copy, and respect applicable law and the source service's terms.
A private-use label does not itself grant copying rights or access to restricted
content. SSYTDLP does not remove DRM, provide paid-service audio access or collect
account passwords/tokens.

Spotify, Deezer and Apple Music links supply public catalog metadata. SSYTDLP
searches YouTube for a matching recording and downloads that recording using
yt-dlp nightly. The catalog provider is shown in the preview, and the actual
YouTube title and URL appear in the log. A match may be a different release;
review the source when exact mastering/version matters.

Direct public media downloads, including SoundCloud and Bandcamp, use the
available formats supplied to yt-dlp. Public streams may be lower quality than
an account-only original. Converting to FLAC or a higher MP3 bitrate cannot
restore lost quality.

Optional lyrics lookup sends track title, artist and duration to LRCLIB using
the open-source syncedlyrics provider. LRCLIB is a community database: coverage,
accuracy and timing vary. SSYTDLP checks identity, duration and content before
saving a match; unsuccessful lyrics lookup retains the media file. The matching
record URL is saved in a .lyrics.json attribution file.

Optional subtitles come from the source. Automatically generated captions can
contain errors. Their language and availability depend on the source.

The UI's Services & personal use settings tab contains the translated notice.
