//! Small libmpv boundary. All handles and rendering stay on the creating thread.
use anyhow::{Context, Result, bail};
use eframe::glow::{self, HasContext};
use std::{
    ffi::{CStr, CString, c_char, c_int, c_void},
    path::PathBuf,
    ptr,
};

type Create = unsafe extern "C" fn() -> *mut c_void;
type Init = unsafe extern "C" fn(*mut c_void) -> c_int;
type Set = unsafe extern "C" fn(*mut c_void, *const c_char, *const c_char) -> c_int;
type Get = unsafe extern "C" fn(*mut c_void, *const c_char) -> *mut c_char;
type GetNode = unsafe extern "C" fn(*mut c_void, *const c_char, c_int, *mut c_void) -> c_int;
type FreeNode = unsafe extern "C" fn(*mut Node);
type Free = unsafe extern "C" fn(*mut c_void);
type ErrorString = unsafe extern "C" fn(c_int) -> *const c_char;
type Cmd = unsafe extern "C" fn(*mut c_void, u64, *const *const c_char) -> c_int;
type Wait = unsafe extern "C" fn(*mut c_void, f64) -> *const Event;
type Destroy = unsafe extern "C" fn(*mut c_void);
type RenderCreate = unsafe extern "C" fn(*mut *mut c_void, *mut c_void, *mut Param) -> c_int;
type Render = unsafe extern "C" fn(*mut c_void, *mut Param) -> c_int;
type Update = unsafe extern "C" fn(*mut c_void) -> u64;
type RenderFree = unsafe extern "C" fn(*mut c_void);
#[repr(C)]
struct Param {
    kind: c_int,
    data: *mut c_void,
}
#[repr(C)]
struct Event {
    id: c_int,
    error: c_int,
    userdata: u64,
    data: *mut c_void,
}
#[repr(C)]
struct EndFile {
    reason: c_int,
    error: c_int,
    playlist_entry_id: i64,
    playlist_insert_id: i64,
    playlist_insert_num_entries: c_int,
}
#[repr(C)]
union NodeValue {
    string: *const c_char,
    flag: c_int,
    int: i64,
    double: f64,
    list: *const NodeList,
}
#[repr(C)]
struct Node {
    value: NodeValue,
    format: c_int,
}
#[repr(C)]
struct NodeList {
    num: c_int,
    values: *const Node,
    keys: *const *const c_char,
}
#[repr(C)]
struct GlInit {
    get: unsafe extern "C" fn(*mut c_void, *const c_char) -> *const c_void,
    ctx: *mut c_void,
}
#[repr(C)]
struct Fbo {
    fbo: c_int,
    w: c_int,
    h: c_int,
    format: c_int,
}
struct Resolver {
    library: libloading::Library,
    extension: Option<unsafe extern "system" fn(*const u8) -> *const c_void>,
}
unsafe extern "C" fn resolve(ctx: *mut c_void, name: *const c_char) -> *const c_void {
    // SAFETY: the boxed resolver outlives libmpv's rendering context.
    unsafe {
        let r = &*(ctx as *const Resolver);
        if let Some(get) = r.extension {
            let p = get(name.cast());
            if !p.is_null() && (p as usize) > 3 && (p as isize) != -1 {
                return p;
            }
        }
        r.library
            .get::<*const c_void>(CStr::from_ptr(name).to_bytes_with_nul())
            .map(|s| *s)
            .unwrap_or(ptr::null())
    }
}
#[derive(Clone, Debug, Default)]
pub struct Track {
    pub id: i64,
    pub kind: String,
    pub title: String,
    pub language: String,
}
#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    pub discontinuity: u64,
    pub position: f64,
    pub duration: f64,
    pub paused: bool,
    pub path: String,
    pub index: usize,
    pub eof: bool,
    pub video: bool,
    pub tracks: Vec<Track>,
    pub error: String,
}
pub struct Player {
    _library: libloading::Library,
    handle: *mut c_void,
    render: *mut c_void,
    _resolver: Option<Box<Resolver>>,
    set: Set,
    get: Get,
    get_node: GetNode,
    free_node: FreeNode,
    free: Free,
    command: Cmd,
    wait: Wait,
    destroy: Destroy,
    render_fn: Render,
    render_update: Update,
    render_free: RenderFree,
    pub snapshot: Snapshot,
    pub texture: Option<glow::NativeTexture>,
    fbo: Option<glow::NativeFramebuffer>,
    pub texture_id: Option<eframe::egui::TextureId>,
    pub frames: u64,
    software: bool,
    closed: bool,
    pending_seek: Option<f64>,
    pending_index: usize,
    error_string: ErrorString,
    next_command: u64,
    commands: std::collections::HashMap<u64, String>,
    loaded: bool,
    pub seek_commands: u64,
    _thread: std::marker::PhantomData<std::rc::Rc<()>>,
}
impl Player {
    pub fn new(gl: Option<&glow::Context>, silent: bool) -> Result<Self> {
        let mut candidates = Vec::new();
        if let Some(p) = std::env::var_os("SSMP_LIBMPV") {
            candidates.push(PathBuf::from(p))
        }
        let names: Vec<&str> = if cfg!(windows) {
            vec!["libmpv-2.dll"]
        } else if cfg!(target_os = "macos") {
            vec![
                "libmpv.2.dylib",
                "/opt/homebrew/lib/libmpv.2.dylib",
                "/usr/local/lib/libmpv.2.dylib",
            ]
        } else {
            vec!["libmpv.so.2", "libmpv.so"]
        };
        for name in names {
            if let Some(dir) = std::env::current_exe()?.parent() {
                candidates.push(dir.join("tools/mpv").join(name));
                candidates.push(dir.join(name));
            }
            candidates.push(
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("tools/mpv")
                    .join(name),
            );
            candidates.push(name.into());
        }
        let mut loaded = None;
        for path in candidates {
            if let Ok(lib) = unsafe { libloading::Library::new(path) } {
                loaded = Some(lib);
                break;
            }
        }
        let lib = loaded.context(
            "Playback engine missing. Install libmpv or use the complete SSMP portable package.",
        )?;
        unsafe {
            let create = *lib.get::<Create>(b"mpv_create\0")?;
            let init = *lib.get::<Init>(b"mpv_initialize\0")?;
            let set_option = *lib.get::<Set>(b"mpv_set_option_string\0")?;
            let rc = *lib.get::<RenderCreate>(b"mpv_render_context_create\0")?;
            let handle = create();
            let error_string = *lib.get::<ErrorString>(b"mpv_error_string\0")?;
            if handle.is_null() {
                bail!("mpv_create failed")
            }
            let mut p = Self {
                handle,
                render: ptr::null_mut(),
                _resolver: None,
                set: *lib.get(b"mpv_set_property_string\0")?,
                get: *lib.get(b"mpv_get_property_string\0")?,
                get_node: *lib.get(b"mpv_get_property\0")?,
                free_node: *lib.get(b"mpv_free_node_contents\0")?,
                free: *lib.get(b"mpv_free\0")?,
                command: *lib.get(b"mpv_command_async\0")?,
                wait: *lib.get(b"mpv_wait_event\0")?,
                destroy: *lib.get(b"mpv_terminate_destroy\0")?,
                render_fn: *lib.get(b"mpv_render_context_render\0")?,
                render_update: *lib.get(b"mpv_render_context_update\0")?,
                render_free: *lib.get(b"mpv_render_context_free\0")?,
                _library: lib,
                snapshot: Snapshot::default(),
                texture: None,
                fbo: None,
                texture_id: None,
                frames: 0,
                software: gl.is_none(),
                closed: false,
                pending_seek: None,
                pending_index: 0,
                error_string,
                next_command: 1,
                commands: Default::default(),
                loaded: false,
                seek_commands: 0,
                _thread: Default::default(),
            };
            for (k, v) in [
                ("config", "no"),
                ("terminal", "no"),
                ("idle", "yes"),
                ("keep-open", "yes"),
                ("vo", "libmpv"),
                ("audio-display", "no"),
                ("cover-art-auto", "no"),
                ("input-default-bindings", "no"),
                ("input-vo-keyboard", "no"),
                ("osc", "no"),
                ("ytdl", "no"),
                ("gapless-audio", "yes"),
                ("cache", "yes"),
            ] {
                let k = CString::new(k)?;
                let v = CString::new(v)?;
                if set_option(handle, k.as_ptr(), v.as_ptr()) < 0 {
                    bail!("Unsupported mpv option: {}", k.to_string_lossy())
                }
            }
            if silent {
                let _ = set_option(handle, c"ao".as_ptr(), c"null".as_ptr());
            }
            if init(handle) < 0 {
                bail!("Playback engine initialization failed")
            }
            let mut params = vec![];
            let mut glinit = GlInit {
                get: resolve,
                ctx: ptr::null_mut(),
            };
            if let Some(gl) = gl {
                let (name, ext) = if cfg!(windows) {
                    ("opengl32.dll", Some(b"wglGetProcAddress\0".as_slice()))
                } else if cfg!(target_os = "macos") {
                    ("/System/Library/Frameworks/OpenGL.framework/OpenGL", None)
                } else {
                    ("libGL.so.1", Some(b"glXGetProcAddressARB\0".as_slice()))
                };
                let library = libloading::Library::new(name)?;
                let extension = ext.and_then(|n| library.get(n).ok().map(|s| *s));
                let mut resolver = Box::new(Resolver { library, extension });
                glinit.ctx = (&mut *resolver as *mut Resolver).cast();
                p._resolver = Some(resolver);
                params.push(Param {
                    kind: 1,
                    data: c"opengl".as_ptr().cast_mut().cast(),
                });
                params.push(Param {
                    kind: 2,
                    data: (&mut glinit as *mut GlInit).cast(),
                });
                let texture = gl.create_texture().map_err(|s| anyhow::anyhow!(s))?;
                gl.bind_texture(glow::TEXTURE_2D, Some(texture));
                gl.tex_parameter_i32(
                    glow::TEXTURE_2D,
                    glow::TEXTURE_MIN_FILTER,
                    glow::LINEAR as i32,
                );
                gl.tex_parameter_i32(
                    glow::TEXTURE_2D,
                    glow::TEXTURE_MAG_FILTER,
                    glow::LINEAR as i32,
                );
                gl.tex_image_2d(
                    glow::TEXTURE_2D,
                    0,
                    glow::RGBA8 as i32,
                    1280,
                    720,
                    0,
                    glow::RGBA,
                    glow::UNSIGNED_BYTE,
                    glow::PixelUnpackData::Slice(None),
                );
                let fbo = gl.create_framebuffer().map_err(|s| anyhow::anyhow!(s))?;
                gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
                gl.framebuffer_texture_2d(
                    glow::FRAMEBUFFER,
                    glow::COLOR_ATTACHMENT0,
                    glow::TEXTURE_2D,
                    Some(texture),
                    0,
                );
                gl.bind_framebuffer(glow::FRAMEBUFFER, None);
                gl.bind_texture(glow::TEXTURE_2D, None);
                p.texture = Some(texture);
                p.fbo = Some(fbo);
            } else {
                params.push(Param {
                    kind: 1,
                    data: c"sw".as_ptr().cast_mut().cast(),
                })
            }
            params.push(Param {
                kind: 0,
                data: ptr::null_mut(),
            });
            if rc(&mut p.render, handle, params.as_mut_ptr()) < 0 {
                bail!("Video renderer initialization failed")
            }
            Ok(p)
        }
    }
    pub fn set(&mut self, key: &str, value: &str) {
        let result = (|| -> Result<()> {
            let k = CString::new(key)?;
            let v = CString::new(value)?;
            if unsafe { (self.set)(self.handle, k.as_ptr(), v.as_ptr()) } < 0 {
                bail!("Could not set {key}")
            }
            Ok(())
        })();
        if let Err(e) = result {
            self.snapshot.error = e.to_string()
        }
    }
    pub fn get(&self, key: &str) -> String {
        let Ok(key) = CString::new(key) else {
            return String::new();
        };
        unsafe {
            let p = (self.get)(self.handle, key.as_ptr());
            if p.is_null() {
                String::new()
            } else {
                let s = CStr::from_ptr(p).to_string_lossy().into_owned();
                (self.free)(p.cast());
                s
            }
        }
    }
    pub fn command(&mut self, args: &[&str]) {
        let result = (|| -> Result<()> {
            let strings: Vec<CString> = args
                .iter()
                .map(|s| CString::new(*s))
                .collect::<std::result::Result<_, _>>()?;
            let mut pointers: Vec<_> = strings.iter().map(|s| s.as_ptr()).collect();
            pointers.push(ptr::null());
            let id = self.next_command;
            self.next_command += 1;
            let code = unsafe { (self.command)(self.handle, id, pointers.as_ptr()) };
            if code < 0 {
                bail!(
                    "{}: {}",
                    args.first().unwrap_or(&"command"),
                    self.error_text(code)
                )
            }
            self.commands
                .insert(id, args.first().unwrap_or(&"command").to_string());
            Ok(())
        })();
        if let Err(e) = result {
            self.snapshot.error = e.to_string()
        }
    }
    pub fn load(&mut self, paths: &[String], index: usize, position: f64, token: Option<&str>) {
        if let Some(path) = paths.get(index) {
            self.load_selected(path, index, position, token);
        }
    }
    pub fn load_selected(&mut self, path: &str, index: usize, position: f64, token: Option<&str>) {
        self.snapshot.error.clear();
        self.commands.clear();
        self.snapshot.eof = false;
        self.loaded = false;
        self.set(
            "http-header-fields",
            &token
                .map(|t| format!("Authorization: Bearer {t}"))
                .unwrap_or_default(),
        );
        // SSMP owns the queue. Loading just the selected track avoids playlist-pos
        // commands racing the asynchronous replacement of mpv's internal playlist.
        self.command(&["loadfile", path, "replace"]);
        self.pending_seek = if position > 0. { Some(position) } else { None };
        self.pending_index = index;
        self.snapshot.index = index;
        self.command(&["set", "pause", "no"]);
    }
    pub fn toggle(&mut self) {
        self.command(&["cycle", "pause"])
    }
    pub fn seek(&mut self, seconds: f64) {
        if !seconds.is_finite() || !self.loaded {
            return;
        }
        self.seek_commands += 1;
        self.command(&[
            "seek",
            &seconds
                .max(0.)
                .min(self.snapshot.duration.max(0.))
                .to_string(),
            "absolute+exact",
        ])
    }
    fn error_text(&self, code: i32) -> String {
        unsafe {
            let p = (self.error_string)(code);
            if p.is_null() {
                format!("mpv {code}")
            } else {
                CStr::from_ptr(p).to_string_lossy().into_owned()
            }
        }
    }
    pub fn property_json(&self, key: &str) -> serde_json::Value {
        let Ok(key) = CString::new(key) else {
            return serde_json::Value::Null;
        };
        unsafe {
            let mut node = Node {
                value: NodeValue { int: 0 },
                format: 0,
            };
            if (self.get_node)(
                self.handle,
                key.as_ptr(),
                6,
                (&mut node as *mut Node).cast(),
            ) < 0
            {
                return serde_json::Value::Null;
            }
            let value = node_json(&node);
            (self.free_node)(&mut node);
            value
        }
    }
    pub fn tick(&mut self) {
        unsafe {
            loop {
                let event = (self.wait)(self.handle, 0.);
                if event.is_null() || (*event).id == 0 {
                    break;
                }
                if (*event).id == 8 {
                    self.loaded = true;
                    self.snapshot.error.clear();
                } else if (*event).id == 20 || (*event).id == 21 {
                    self.snapshot.discontinuity += 1;
                } else if (*event).id == 7 && !(*event).data.is_null() {
                    let end = &*((*event).data as *const EndFile);
                    if end.reason == 4 && end.error < 0 {
                        self.snapshot.error = format!("Playback: {}", self.error_text(end.error));
                    }
                } else if (*event).id == 5 {
                    if let Some(command) = self.commands.remove(&(*event).userdata) {
                        // Only an actual failed command is an error; obsolete replies
                        // from replaced files must not leave an idle/startup banner.
                        if (*event).error < 0 && (*event).error != -10 {
                            self.snapshot.error =
                                format!("{command}: {}", self.error_text((*event).error));
                        }
                    }
                }
            }
        }
        self.snapshot.position = self.get("time-pos").parse().unwrap_or(0.);
        self.snapshot.duration = self.get("duration").parse().unwrap_or(0.);
        self.snapshot.paused = self.get("pause") == "yes";
        self.snapshot.eof = self.get("eof-reached") == "yes";
        self.snapshot.path = self.get("path");
        if self.loaded && self.snapshot.duration > 0. {
            if let Some(position) = self.pending_seek.take() {
                self.seek(position);
            }
        }
        self.snapshot.video = !self.get("video-format").is_empty();
        unsafe {
            let mut node = Node {
                value: NodeValue { int: 0 },
                format: 0,
            };
            if (self.get_node)(
                self.handle,
                c"track-list".as_ptr(),
                6,
                (&mut node as *mut Node).cast(),
            ) >= 0
            {
                let v = node_json(&node);
                self.snapshot.tracks = v
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .map(|v| Track {
                                id: v["id"].as_i64().unwrap_or(0),
                                kind: v["type"].as_str().unwrap_or("").into(),
                                title: v["title"].as_str().unwrap_or("").into(),
                                language: v["lang"].as_str().unwrap_or("").into(),
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                (self.free_node)(&mut node);
            }
        }
    }
    pub fn render_gl(&mut self, gl: &glow::Context) {
        if self.software || self.closed {
            return;
        }
        unsafe {
            if (self.render_update)(self.render) & 1 != 0 {
                let mut fbo = Fbo {
                    fbo: self.fbo.unwrap().0.get() as i32,
                    w: 1280,
                    h: 720,
                    format: glow::RGBA8 as i32,
                };
                let mut flip = 0;
                let mut block = 0;
                let mut params = [
                    Param {
                        kind: 3,
                        data: (&mut fbo as *mut Fbo).cast(),
                    },
                    Param {
                        kind: 4,
                        data: (&mut flip as *mut i32).cast(),
                    },
                    Param {
                        kind: 12,
                        data: (&mut block as *mut i32).cast(),
                    },
                    Param {
                        kind: 0,
                        data: ptr::null_mut(),
                    },
                ];
                if (self.render_fn)(self.render, params.as_mut_ptr()) >= 0 {
                    self.frames += 1;
                }
                gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            }
        }
    }
    pub fn render_software(&mut self) -> Result<Vec<u8>> {
        Ok(self
            .render_software_size(320, 180)?
            .unwrap_or_else(|| vec![0; 320 * 180 * 4]))
    }
    pub fn video_frame_software(&mut self) -> Result<Option<Vec<u8>>> {
        self.render_software_size(1280, 720)
    }
    fn render_software_size(&mut self, width: usize, height: usize) -> Result<Option<Vec<u8>>> {
        if !self.software || self.closed || unsafe { (self.render_update)(self.render) & 1 == 0 } {
            return Ok(None);
        }
        let mut pixels = vec![0u8; width * height * 4];
        let mut size = [width as i32, height as i32];
        let mut stride = width * 4;
        let mut block = 0;
        unsafe {
            let mut params = [
                Param {
                    kind: 17,
                    data: size.as_mut_ptr().cast(),
                },
                Param {
                    kind: 18,
                    data: c"rgb0".as_ptr().cast_mut().cast(),
                },
                Param {
                    kind: 19,
                    data: (&mut stride as *mut usize).cast(),
                },
                Param {
                    kind: 20,
                    data: pixels.as_mut_ptr().cast(),
                },
                Param {
                    kind: 12,
                    data: (&mut block as *mut i32).cast(),
                },
                Param {
                    kind: 0,
                    data: ptr::null_mut(),
                },
            ];
            if (self.render_fn)(self.render, params.as_mut_ptr()) < 0 {
                bail!("Frame rendering failed")
            }
            self.frames += 1;
        }
        Ok(Some(pixels))
    }
    pub fn close(&mut self, gl: Option<&glow::Context>) {
        if self.closed {
            return;
        }
        unsafe {
            if !self.render.is_null() {
                (self.render_free)(self.render)
            }
            if !self.handle.is_null() {
                (self.destroy)(self.handle)
            }
            if let Some(gl) = gl {
                if let Some(f) = self.fbo.take() {
                    gl.delete_framebuffer(f)
                }
            }
        }
        self.closed = true;
    }
}
impl Drop for Player {
    fn drop(&mut self) {
        self.close(None)
    }
}
unsafe fn node_json(n: &Node) -> serde_json::Value {
    unsafe {
        use serde_json::{Value, json};
        match n.format {
            1 => {
                if n.value.string.is_null() {
                    Value::Null
                } else {
                    json!(CStr::from_ptr(n.value.string).to_string_lossy())
                }
            }
            3 => json!(n.value.flag != 0),
            4 => json!(n.value.int),
            5 => json!(n.value.double),
            7 | 8 => {
                let l = n.value.list;
                if l.is_null() {
                    return Value::Null;
                }
                let l = &*l;
                if l.num <= 0 {
                    return if n.format == 7 {
                        Value::Array(vec![])
                    } else {
                        Value::Object(Default::default())
                    };
                }
                let values = std::slice::from_raw_parts(l.values, l.num.max(0) as usize);
                if n.format == 7 {
                    Value::Array(values.iter().map(|v| node_json(v)).collect())
                } else {
                    let mut map = serde_json::Map::new();
                    for (i, v) in values.iter().enumerate() {
                        let key = CStr::from_ptr(*l.keys.add(i))
                            .to_string_lossy()
                            .into_owned();
                        map.insert(key, node_json(v));
                    }
                    Value::Object(map)
                }
            }
            _ => Value::Null,
        }
    }
}
pub fn self_test(path: &str) -> Result<()> {
    self_test_authenticated(path, None)
}
pub fn interaction_test(path: &str) -> Result<()> {
    fn until(p: &mut Player, ok: impl Fn(&Snapshot) -> bool) -> Result<()> {
        let start = std::time::Instant::now();
        while start.elapsed().as_secs_f32() < 4. {
            p.tick();
            let _ = p.render_software()?;
            if !p.snapshot.error.is_empty() {
                bail!("{}", p.snapshot.error);
            }
            if ok(&p.snapshot) {
                return Ok(());
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        bail!("Playback state timed out: {:?}", p.snapshot)
    }
    let path = std::fs::canonicalize(path)?.to_string_lossy().into_owned();
    let mut p = Player::new(None, true)?;
    p.load(&[path.clone()], 0, 0., None);
    until(&mut p, |s| s.position > 0.3)?;
    println!("INITIAL_PLAYBACK=true");
    p.set("pause", "yes");
    p.seek(2.);
    // Audio output is packetized; the paused clock can lag the target by one
    // output buffer. Keep a bounded tolerance rather than relying on CPU timing.
    until(&mut p, |s| (s.position - 2.).abs() < 0.25 && s.paused)?;
    println!("PAUSED_SEEK_PRESERVED=true");
    p.set("pause", "no");
    for target in [0.5, 1.5, 3.0] {
        p.seek(target);
        until(&mut p, |s| (s.position - target).abs() < 0.2 && !s.paused)?;
    }
    let before = p.snapshot.position;
    std::thread::sleep(std::time::Duration::from_millis(250));
    p.tick();
    if p.snapshot.position <= before {
        bail!("Playback did not continue after seek");
    }
    println!(
        "SEEK_PLAYBACK_CONTINUES=true SEEK_COMMANDS={}",
        p.seek_commands
    );
    for _ in 0..3 {
        p.load(&[path.clone()], 0, 0., None);
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    until(&mut p, |s| s.position > 0.25)?;
    p.load(
        &["nonexistent-ignored-queue-entry".into(), path.clone()],
        1,
        1.25,
        None,
    );
    until(&mut p, |s| s.position >= 1.25 && s.index == 1)?;
    println!(
        "RAPID_REPLACE_AND_NONZERO_QUEUE_INDEX=true ERROR_EMPTY={}",
        p.snapshot.error.is_empty()
    );
    p.command(&["loadfile", "nonexistent-ssmp-regression-file", "replace"]);
    let start = std::time::Instant::now();
    while start.elapsed().as_secs_f32() < 2. && p.snapshot.error.is_empty() {
        p.tick();
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    if p.snapshot.error.is_empty() {
        bail!("Missing file error was not reported");
    }
    p.load(&[path], 0, 0., None);
    until(&mut p, |s| s.position > 0.25)?;
    println!("GENUINE_ERROR_AND_RECOVERY=true");
    p.close(None);
    Ok(())
}
pub fn music_video_test(audio: &str, video: &str) -> Result<()> {
    let audio = std::fs::canonicalize(audio)?.to_string_lossy().into_owned();
    let video = std::fs::canonicalize(video)?.to_string_lossy().into_owned();
    let mut p = Player::new(None, true)?;
    p.load(&[audio], 0, 0., None);
    let start = std::time::Instant::now();
    while start.elapsed().as_secs_f32() < 1. {
        p.tick();
        let _ = p.render_software()?;
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    p.command(&["video-add", &video, "select"]);
    let mut pixels = false;
    let start = std::time::Instant::now();
    while start.elapsed().as_secs_f32() < 1. {
        p.tick();
        pixels |= p.render_software()?.iter().any(|v| *v != 0);
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    if !p.snapshot.error.is_empty()
        || !pixels
        || !p.snapshot.tracks.iter().any(|t| t.kind == "video")
    {
        bail!("Linked video failed: {:?}", p.snapshot);
    }
    let position = p.snapshot.position;
    p.set("vid", "no");
    std::thread::sleep(std::time::Duration::from_millis(200));
    p.tick();
    if p.snapshot.paused || p.snapshot.position <= position {
        bail!("Hiding video interrupted audio");
    }
    p.set("vid", "auto");
    let start = std::time::Instant::now();
    while start.elapsed().as_secs_f32() < 0.3 {
        p.tick();
        let _ = p.render_software()?;
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    if !p.snapshot.error.is_empty() {
        bail!("{}", p.snapshot.error);
    }
    println!("LINKED_VIDEO_RENDERED=true AUDIO_CONTINUES_WHEN_HIDDEN=true RESTORE_VIDEO=true");
    p.close(None);
    Ok(())
}
pub fn self_test_authenticated(path: &str, token: Option<&str>) -> Result<()> {
    let path = if path.starts_with("http://") || path.starts_with("https://") {
        path.to_string()
    } else {
        std::fs::canonicalize(path)?.to_string_lossy().into_owned()
    };
    let mut p = Player::new(None, true)?;
    p.load(&[path], 0, 0., token);
    let started = std::time::Instant::now();
    let mut advanced = false;
    let mut nonzero = false;
    while started.elapsed().as_secs_f64() < 2.5 {
        p.tick();
        let frame = p.render_software()?;
        nonzero |= frame.iter().any(|b| *b > 0);
        advanced |= p.snapshot.position > 0.2;
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let duration = p.snapshot.duration;
    let tracks = p.snapshot.tracks.len();
    p.seek(0.5);
    std::thread::sleep(std::time::Duration::from_millis(200));
    p.tick();
    println!(
        "PLAYBACK_ADVANCED={advanced} DURATION={duration:.3} TRACKS={tracks} VIDEO_FRAMES={} NONZERO_PIXELS={nonzero} SEEK_POSITION={:.3}",
        p.frames, p.snapshot.position
    );
    if !advanced || duration <= 0. || !p.snapshot.error.is_empty() {
        bail!("Playback test failed: {}", p.snapshot.error)
    }
    p.close(None);
    Ok(())
}
