# herdr-core evidence
Fetched read-only 2026-11-12. Upstream master resolves to 7edc5dfbe3f54ac23ad22313ce32f552f219ff44. Not a source match proven for installed hc.

Command: `curl -fsSL "https://api.github.com/repos/herdrdev/herdr-core/git/trees/7edc5dfbe3f54ac23ad22313ce32f552f219ff44?recursive=1"`
Output: sha=7edc5dfbe3f54ac23ad22313ce32f552f219ff44, truncated=False.
Neither src/integration/assets nor src/detect/manifests exists, including under crates/herdr-core/. No integration or detect directory exists in this tree. The plan references unavailable source paths. Do not infer config installation behavior from their absence.

## status.rs
Source: https://raw.githubusercontent.com/herdrdev/herdr-core/7edc5dfbe3f54ac23ad22313ce32f552f219ff44/crates/herdr-core/src/status.rs
Command: `curl -fsSL https://raw.githubusercontent.com/herdrdev/herdr-core/7edc5dfbe3f54ac23ad22313ce32f552f219ff44/crates/herdr-core/src/status.rs`
```text
1: //! Status facts from OSC 7501, kept per terminal exactly as reported.
2: //! A report replaces its record; `clear` removes a record and its children.
3: //! The turn goes up each time the root record leaves `working`, for any reason.
4: 
5: use crate::terminals::Terminal;
6: use ghostty_vt::StatusReport;
7: use schemars::JsonSchema;
8: use serde::{Deserialize, Serialize};
9: 
10: const MAX_RECORDS: usize = 256;
11: 
12: /// One 7501 record. Words are the program's own; nothing is translated.
13: #[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
14: pub struct Status {
15:     /// idle, working, done, blocked or error.
16:     pub state: String,
17:     /// permission, question or auth (blocked only, when the program said).
18:     #[serde(skip_serializing_if = "Option::is_none")]
19:     pub kind: Option<String>,
20:     /// 0..100 (working or blocked, when the program said).
21:     #[serde(skip_serializing_if = "Option::is_none")]
22:     pub progress: Option<u8>,
23:     #[serde(skip_serializing_if = "String::is_empty", default)]
24:     pub app: String,
25:     #[serde(skip_serializing_if = "String::is_empty", default)]
26:     pub title: String,
27:     #[serde(skip_serializing_if = "String::is_empty", default)]
28:     pub msg: String,
29: }
30: 
31: impl Status {
32:     fn from(r: &StatusReport) -> Status {
33:         Status {
34:             state: r.state.into(),
35:             kind: r.kind.map(Into::into),
36:             progress: r.progress,
37:             app: r.app.clone(),
38:             title: r.title.clone(),
39:             msg: r.message.clone(),
40:         }
41:     }
42: 
43:     /// The report again as OSC 7501, for a new viewer's first paint.
44:     pub fn encode(&self, id: &str) -> String {
45:         let mut fields = vec![format!("state={}", self.state)];
46:         let mut push = |key: &str, value: &str| {
47:             if !value.is_empty() {
48:                 fields.push(format!("{key}={value}"));
49:             }
50:         };
51:         push("id", id);
52:         push("kind", self.kind.as_deref().unwrap_or(""));
53:         push("progress", &self.progress.map(|p| p.to_string()).unwrap_or_default());
54:         push("app", &self.app);
55:         push("title", &base64(self.title.as_bytes()));
56:         push("msg", &base64(self.msg.as_bytes()));
57:         format!("\x1b]7501;{}\x1b\\", fields.join(":"))
58:     }
59: }
60: 
61: /// What a report changed, for events and waiters.
62: #[derive(Default, Debug, PartialEq)]
63: pub struct Change {
64:     pub status: bool,
65:     /// A fresh root report, not a child record or an unchanged recovery replay.
66:     pub root_report: bool,
67:     pub turn_ended: bool,
68:     /// It just got stuck: blocked on the user or errored (wasn't that before).
69:     pub stuck: bool,
70: }
71: 
72: fn working(t: &Terminal) -> bool {
73:     t.status.as_ref().is_some_and(|s| s.state == "working")
74: }
75: 
76: fn end_turn(t: &mut Terminal, was_working: bool) -> bool {
77:     let ended = was_working && !working(t);
78:     if ended {
79:         t.turn += 1;
80:     }
81:     ended
82: }
83: 
84: fn stuck(t: &Terminal) -> Option<&str> {
85:     t.status.as_ref().map(|s| s.state.as_str()).filter(|w| *w == "blocked" || *w == "error")
86: }
87: 
88: fn word(t: &Terminal) -> Option<String> {
89:     t.status.as_ref().map(|s| s.state.clone())
90: }
91: 
92: fn stamp(t: &mut Terminal, was: Option<String>) {
93:     if word(t) != was {
94:         t.status_ms = Some(crate::runtime::now_ms());
95:     }
96: }
97: 
98: pub fn apply(t: &mut Terminal, r: &StatusReport) -> Change {
99:     let (was_working, was_word) = (working(t), word(t));
100:     let was = stuck(t).map(String::from);
101:     t.reported = true;
102:     if r.state == "clear" {
103:         if r.id.is_empty() {
104:             t.status = None;
105:             t.records.clear();
106:         } else {
107:             let child = format!("{}/", r.id);
108:             t.records.retain(|id, _| *id != r.id && !id.starts_with(&child));
109:         }
110:     } else if r.id.is_empty() {
111:         t.status = Some(Status::from(r));
112:     } else {
113:         // Over the limit, the record updated longest ago goes (approximated by first id).
114:         if !t.records.contains_key(&r.id) && t.records.len() >= MAX_RECORDS - 1 {
115:             let oldest = t.records.keys().next().cloned().expect("non-empty");
116:             t.records.remove(&oldest);
117:         }
118:         t.records.insert(r.id.clone(), Status::from(r));
119:     }
120:     stamp(t, was_word);
121:     Change {
122:         status: true,
123:         root_report: r.id.is_empty(),
124:         turn_ended: end_turn(t, was_working),
125:         stuck: stuck(t).is_some() && stuck(t) != was.as_deref(),
126:     }
127: }
128: 
129: /// The program ended: its status facts reset.
130: pub fn reset(t: &mut Terminal) -> Change {
131:     let (was_working, was_word) = (working(t), word(t));
132:     let had = t.status.is_some() || !t.records.is_empty();
133:     t.status = None;
134:     t.records.clear();
135:     stamp(t, was_word);
136:     Change { status: had, turn_ended: end_turn(t, was_working), ..Default::default() }
137: }
138: 
139: fn base64(bytes: &[u8]) -> String {
140:     const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
141:     let mut out = String::new();
142:     for chunk in bytes.chunks(3) {
143:         let n = chunk.iter().enumerate().fold(0u32, |n, (i, b)| n | (*b as u32) << (16 - 8 * i));
144:         for i in 0..4 {
145:             out.push(if i <= chunk.len() { TABLE[(n >> (18 - 6 * i) & 63) as usize] as char } else { '=' });
146:         }
147:     }
148:     out
149: }
150: 
151: #[cfg(test)]
152: mod tests {
153:     use super::*;
154:     use crate::terminals::{NewTerminal, Registry};
155: 
156:     fn terminal() -> Terminal {
157:         let mut r = Registry::default();
158:         let new = NewTerminal { argv: vec!["x".into()], cwd: "/".into(), ..Default::default() };
159:         let id = r.create(new, 0).unwrap();
160:         r.terminals.remove(&id).unwrap()
161:     }
162: 
163:     fn report(state: &'static str, id: &str) -> StatusReport {
164:         StatusReport {
165:             state,
166:             kind: None,
167:             progress: None,
168:             id: id.into(),
169:             app: String::new(),
170:             title: String::new(),
171:             message: String::new(),
172:         }
173:     }
174: 
175:     #[test]
176:     fn the_turn_counts_every_way_of_leaving_working() {
177:         let mut t = terminal();
178:         assert!(!apply(&mut t, &report("idle", "")).turn_ended);
179:         apply(&mut t, &report("working", ""));
180:         assert!(apply(&mut t, &report("done", "")).turn_ended); // finished
181:         apply(&mut t, &report("working", ""));
182:         assert!(apply(&mut t, &report("idle", "")).turn_ended); // interrupted
183:         apply(&mut t, &report("working", ""));
184:         assert!(apply(&mut t, &report("blocked", "")).turn_ended); // asks permission
185:         apply(&mut t, &report("working", ""));
186:         assert!(apply(&mut t, &report("clear", "")).turn_ended); // cleared
187:         apply(&mut t, &report("working", ""));
188:         assert!(!apply(&mut t, &report("working", "sub")).turn_ended, "child records don't count");
189:         assert!(reset(&mut t).turn_ended); // exited
190:         assert_eq!(t.turn, 5);
191:         assert!(t.status.is_none() && t.reported);
192:     }
193: 
194:     #[test]
195:     fn the_status_time_moves_only_when_the_word_changes() {
196:         let mut t = terminal();
197:         assert_eq!(t.status_ms, None, "never said anything");
198:         apply(&mut t, &report("working", ""));
199:         let since = t.status_ms.expect("its first word");
200:         t.status_ms = Some(1);
201:         apply(&mut t, &report("working", ""));
202:         apply(&mut t, &report("done", "sub"));
203:         assert_eq!(t.status_ms, Some(1), "the same word again, or a child record");
204:         apply(&mut t, &report("done", ""));
205:         assert!(t.status_ms >= Some(since));
206:         t.status_ms = Some(1);
207:         reset(&mut t);
208:         assert!(t.status_ms > Some(1), "exiting clears the word");
209:     }
210: 
211:     #[test]
212:     fn getting_stuck_is_news_even_outside_a_turn() {
213:         let mut t = terminal();
214:         apply(&mut t, &report("idle", ""));
215:         assert!(apply(&mut t, &report("error", "")).stuck, "e.g. it failed while starting");
216:         assert!(!apply(&mut t, &report("error", "")).stuck, "once");
217:         assert!(apply(&mut t, &report("blocked", "")).stuck);
218:         assert!(!apply(&mut t, &report("idle", "")).stuck);
219:     }
220: 
221:     #[test]
222:     fn clear_removes_a_record_and_its_children() {
223:         let mut t = terminal();
224:         for id in ["build", "build/test", "buildx", "deploy"] {
225:             apply(&mut t, &report("working", id));
226:         }
227:         apply(&mut t, &report("clear", "build"));
228:         assert_eq!(t.records.keys().collect::<Vec<_>>(), ["buildx", "deploy"]);
229:     }
230: 
231:     #[test]
232:     fn reports_encode_back_to_what_the_parser_reads() {
233:         let mut screen = ghostty_vt::Terminal::new(80, 24, 1 << 20);
234:         let sent = StatusReport {
235:             state: "blocked",
236:             kind: Some("permission"),
237:             progress: Some(40),
238:             id: "a/b".into(),
239:             app: "claude-code".into(),
240:             title: "Run tests".into(),
241:             message: "Apply these changes?".into(),
242:         };
243:         screen.write(Status::from(&sent).encode("a/b").as_bytes());
244:         assert_eq!(screen.take_status_reports(), vec![sent]);
245:         assert_eq!(base64(b"Apply?"), "QXBwbHk/");
246:         assert_eq!(base64(b"ab"), "YWI=");
247:     }
248: }
```
## foreground.rs
Source: https://raw.githubusercontent.com/herdrdev/herdr-core/7edc5dfbe3f54ac23ad22313ce32f552f219ff44/crates/herdr-core/src/foreground.rs
Command: `curl -fsSL https://raw.githubusercontent.com/herdrdev/herdr-core/7edc5dfbe3f54ac23ad22313ce32f552f219ff44/crates/herdr-core/src/foreground.rs`
```text
1: //! What runs in a terminal right now: its foreground process and that
2: //! process's folder. Raw facts, read about twice a second; clients name and
3: //! draw programs themselves.
4: 
5: use crate::runtime::Core;
6: use schemars::JsonSchema;
7: use serde::{Deserialize, Serialize};
8: use std::sync::Arc;
9: use std::time::Duration;
10: 
11: #[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
12: pub struct Foreground {
13:     /// The foreground process group's leader.
14:     pub pid: i32,
15:     /// Its name as the OS has it (`node`, `vim`, `sh`).
16:     pub name: String,
17:     pub argv: Vec<String>,
18:     /// Its working directory now (resolved by the OS).
19:     pub cwd: String,
20: }
21: 
22: /// The facts about `pid`, if it is still there to read.
23: #[cfg(target_os = "linux")]
24: pub fn about(pid: i32) -> Option<Foreground> {
25:     let name = std::fs::read_to_string(format!("/proc/{pid}/comm")).ok()?.trim_end().to_string();
26:     let cmdline = std::fs::read(format!("/proc/{pid}/cmdline")).ok()?;
27:     let argv =
28:         cmdline.split(|b| *b == 0).filter(|a| !a.is_empty()).map(|a| String::from_utf8_lossy(a).into_owned()).collect();
29:     let cwd = std::fs::read_link(format!("/proc/{pid}/cwd")).ok()?.display().to_string();
30:     Some(Foreground { pid, name, argv, cwd })
31: }
32: 
33: /// A pseudo console has no foreground group: this is the terminal's first
34: /// process, with the folder and command line Windows keeps in its memory.
35: /// None for a 32-bit process, whose own copy of them lives elsewhere.
36: #[cfg(windows)]
37: pub fn about(pid: i32) -> Option<Foreground> {
38:     use std::ffi::c_void;
39:     use std::mem::{MaybeUninit, size_of};
40:     use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
41:     use windows_sys::Wdk::System::Threading::{NtQueryInformationProcess, ProcessBasicInformation};
42:     use windows_sys::Win32::Foundation::{HANDLE, UNICODE_STRING};
43:     use windows_sys::Win32::System::Diagnostics::Debug::ReadProcessMemory;
44:     use windows_sys::Win32::System::Threading::*;
45: 
46:     let raw = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_VM_READ, 0, pid as u32) };
47:     if raw.is_null() {
48:         return None;
49:     }
50:     let process = unsafe { OwnedHandle::from_raw_handle(raw) };
51:     let h = process.as_raw_handle() as HANDLE;
52:     let mut wow64 = 0;
53:     if unsafe { IsWow64Process(h, &mut wow64) } == 0 || wow64 != 0 {
54:         return None;
55:     }
56:     let read = |at: *const c_void, into: *mut c_void, len: usize| {
57:         let mut n = 0;
58:         unsafe { ReadProcessMemory(h, at, into, len, &mut n) != 0 && n == len }
59:     };
60:     fn value<T>(read: impl Fn(*const c_void, *mut c_void, usize) -> bool, at: *const c_void) -> Option<T> {
61:         let mut v = MaybeUninit::<T>::uninit();
62:         read(at, v.as_mut_ptr().cast(), size_of::<T>()).then(|| unsafe { v.assume_init() })
63:     }
64:     let text = |s: UNICODE_STRING| -> Option<Vec<u16>> {
65:         let mut buf = vec![0u16; s.Length as usize / 2];
66:         read(s.Buffer.cast(), buf.as_mut_ptr().cast(), buf.len() * 2).then_some(buf)
67:     };
68: 
69:     let mut basic: PROCESS_BASIC_INFORMATION = unsafe { std::mem::zeroed() };
70:     let size = size_of::<PROCESS_BASIC_INFORMATION>() as u32;
71:     let status =
72:         unsafe { NtQueryInformationProcess(h, ProcessBasicInformation, (&raw mut basic).cast(), size, &mut 0) };
73:     if status != 0 || basic.PebBaseAddress.is_null() {
74:         return None;
75:     }
76:     let peb: PEB = value(read, basic.PebBaseAddress.cast())?;
77:     let params: RTL_USER_PROCESS_PARAMETERS = value(read, peb.ProcessParameters.cast())?;
78:     // CurrentDirectory.DosPath: right after the standard handles, in what the SDK calls Reserved2.
79:     let dir: UNICODE_STRING = value(read, peb.ProcessParameters.cast::<u8>().wrapping_add(0x38).cast())?;
80:     let mut cwd = String::from_utf16(&text(dir)?).ok()?;
81:     if cwd.len() > 3 && cwd.ends_with('\\') {
82:         cwd.pop(); // Windows keeps C:\dir\; only a drive's root has one
83:     }
84:     let image = String::from_utf16(&text(params.ImagePathName)?).ok()?;
85:     let name = image.rsplit('\\').next().unwrap_or(&image).to_string();
86: 
87:     let argv = crate::platform::split_command_line(text(params.CommandLine)?)
88:         .ok()?
89:         .into_iter()
90:         .map(|arg| arg.to_string_lossy().into_owned())
91:         .collect();
92:     Some(Foreground { pid, name, argv, cwd })
93: }
94: 
95: #[cfg(target_os = "macos")]
96: pub fn about(pid: i32) -> Option<Foreground> {
97:     let mut name = [0u8; 256];
98:     let n = unsafe { libc::proc_name(pid, name.as_mut_ptr().cast(), name.len() as u32) };
99:     let name = String::from_utf8_lossy(&name[..n.max(0) as usize]).into_owned();
100:     let mut vnode: libc::proc_vnodepathinfo = unsafe { std::mem::zeroed() };
101:     let size = std::mem::size_of::<libc::proc_vnodepathinfo>() as libc::c_int;
102:     if unsafe { libc::proc_pidinfo(pid, libc::PROC_PIDVNODEPATHINFO, 0, (&raw mut vnode).cast(), size) } != size {
103:         return None;
104:     }
105:     let path = unsafe { std::ffi::CStr::from_ptr(vnode.pvi_cdir.vip_path.as_ptr().cast()) };
106:     let (argv, _) = procargs(pid)?;
107:     Some(Foreground { pid, name, argv, cwd: path.to_string_lossy().into_owned() })
108: }
109: 
110: /// KERN_PROCARGS2: argc, the executable path, the arguments, then the
111: /// environment, each NUL-terminated. Returns (arguments, environment).
112: #[cfg(target_os = "macos")]
113: pub fn procargs(pid: i32) -> Option<(Vec<String>, Vec<u8>)> {
114:     let mut mib = [libc::CTL_KERN, libc::KERN_PROCARGS2, pid];
115:     let mut size: libc::size_t = 0;
116:     let call = |buf: *mut libc::c_void, size: &mut libc::size_t, mib: &mut [i32; 3]| unsafe {
117:         libc::sysctl(mib.as_mut_ptr(), 3, buf, size, std::ptr::null_mut(), 0)
118:     };
119:     if call(std::ptr::null_mut(), &mut size, &mut mib) != 0 || size < 4 {
120:         return None;
121:     }
122:     let mut buf = vec![0u8; size];
123:     if call(buf.as_mut_ptr().cast(), &mut size, &mut mib) != 0 {
124:         return None;
125:     }
126:     buf.truncate(size);
127:     let argc = i32::from_ne_bytes(buf.get(..4)?.try_into().ok()?) as usize;
128:     let mut rest = &buf[4..];
129:     let one = |r: &[u8]| -> Option<usize> { r.iter().position(|b| *b == 0) };
130:     rest = &rest[one(rest)? + 1..]; // the executable path
131:     while rest.first() == Some(&0) {
132:         rest = &rest[1..]; // its padding
133:     }
134:     let mut argv = Vec::with_capacity(argc);
135:     for _ in 0..argc {
136:         let end = one(rest)?;
137:         argv.push(String::from_utf8_lossy(&rest[..end]).into_owned());
138:         rest = &rest[end + 1..];
139:     }
140:     Some((argv, rest.to_vec()))
141: }
142: 
143: fn shell_root(root: &Foreground) -> bool {
144:     // Command and script wrappers can share their foreground group with a live agent.
145:     if root.argv.iter().skip(1).any(|arg| {
146:         (!arg.starts_with('-') && !arg.starts_with('+'))
147:             || matches!(arg.split('=').next(), Some("--command" | "--commands"))
148:             || (arg.starts_with('-') && !arg.starts_with("--") && arg[1..].contains('c'))
149:     }) {
150:         return false;
151:     }
152:     let program =
153:         if root.name.is_empty() { root.argv.first().map(String::as_str).unwrap_or_default() } else { &root.name };
154:     let name = program.rsplit('/').next().unwrap_or(program);
155:     matches!(name, "sh" | "bash" | "zsh" | "fish" | "nu" | "dash" | "ksh" | "tcsh")
156: }
157: 
158: impl Core {
159:     /// Keeps every live terminal's `foreground` current, with an event on each change.
160:     pub async fn watch_foreground(self: Arc<Self>) {
161:         let ms = std::env::var("HC_FOREGROUND_MS").ok().and_then(|v| v.parse().ok()).unwrap_or(500);
162:         loop {
163:             tokio::time::sleep(Duration::from_millis(ms)).await;
164:             let live: Vec<u32> = self.lock().reg.terminals.values().filter(|t| t.is_live()).map(|t| t.id()).collect();
165:             for id in live {
166:                 let pgid = self.foreground(id).await;
167:                 let now = (pgid > 0).then(|| about(pgid)).flatten();
168:                 let mut st = self.lock();
169:                 let Some(t) = st.reg.terminals.get_mut(&id).filter(|t| t.is_live()) else {
170:                     continue;
171:                 };
172:                 let at_shell = crate::platform::foreground_tracks_jobs()
173:                     && now.as_ref().is_some_and(|n| Some(n.pid as u32) == t.pid && shell_root(n));
174:                 let change = (at_shell && (t.status.is_some() || !t.records.is_empty())).then(|| {
175:                     t.first_task_turn = None;
176:                     crate::status::reset(t)
177:                 });
178:                 let foreground_changed = t.foreground != now;
179:                 if let Some(f) = &now {
180:                     t.last_cwd = Some(f.cwd.clone());
181:                 }
182:                 t.foreground = now.clone();
183:                 if change.is_some() || foreground_changed {
184:                     self.save_soon();
185:                 }
186:                 if let Some(change) = change {
187:                     self.status_changed(&mut st, id, change);
188:                 }
189:                 if foreground_changed {
190:                     self.emit(&mut st, "foreground", id, serde_json::to_value(&now).unwrap_or_default());
191:                 }
192:             }
193:         }
194:     }
195: }
196: 
197: #[cfg(test)]
198: mod tests {
199:     use super::*;
200: 
201:     #[test]
202:     fn command_flags_keep_shell_roots_from_clearing_live_status() {
203:         for shell in ["sh", "bash", "zsh", "fish", "nu", "dash", "ksh", "tcsh"] {
204:             let mut root = Foreground { pid: 1, name: shell.into(), argv: vec![shell.into()], cwd: "/".into() };
205:             assert!(shell_root(&root), "{shell}");
206:             for arg in ["-i", "-li", "--login", "--no-config"] {
207:                 root.argv = vec![shell.into(), arg.into()];
208:                 assert!(shell_root(&root), "{shell} {arg}");
209:             }
210:             for arg in ["-c", "-cecho", "-ic echo", "--command", "--command=echo", "--commands", "--commands=echo"] {
211:                 root.argv = vec![shell.into(), arg.into()];
212:                 assert!(!shell_root(&root), "{shell} {arg}");
213:             }
214:         }
215:     }
216: }
```
## lib.rs
Source: https://raw.githubusercontent.com/herdrdev/herdr-core/7edc5dfbe3f54ac23ad22313ce32f552f219ff44/crates/herdr-core/src/lib.rs
Command: `curl -fsSL https://raw.githubusercontent.com/herdrdev/herdr-core/7edc5dfbe3f54ac23ad22313ce32f552f219ff44/crates/herdr-core/src/lib.rs`
```text
1: pub mod addr;
2: pub mod api;
3: pub mod attach;
4: pub mod cli;
5: pub mod client;
6: pub mod config;
7: pub mod events;
8: pub mod exec;
9: pub mod files;
10: pub mod foreground;
11: pub mod holder;
12: pub mod holder_proto;
13: pub mod input;
14: mod kept;
15: pub mod keys;
16: pub mod locale;
17: pub mod machines;
18: pub mod notes;
19: pub mod paths;
20: pub mod picker;
21: pub mod platform;
22: pub mod put;
23: pub mod relay;
24: pub mod releases;
25: pub mod remote;
26: mod restart;
27: pub mod rpc;
28: pub mod runtime;
29: pub mod server;
30: pub mod status;
31: pub mod stitch;
32: pub mod sweep;
33: pub mod templates;
34: pub mod terminals;
35: pub mod viewers;
36: pub mod wait;
37: pub mod wire;
```
