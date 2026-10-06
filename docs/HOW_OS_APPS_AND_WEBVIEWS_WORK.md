# How an OS, a Binary, and a WebView Actually Work

> A bottom-up primer for a backend engineer who has never built a desktop app.
> Written specifically as background reading for `SYSTEM_ARCHITECTURE_AND_LEARNING.md`.

---

## Layer 0 — How to read this doc

**Who I assumed you are:** you ship NestJS services to Cloud Run, write SQL against Postgres, cache things in Redis, build React UIs that talk to your API over HTTP. You've used Docker, GitHub Actions, and the browser DevTools. You've **not** spent time below that — you have not written C, looked at a Mach-O binary, traced a syscall, or thought about what a "process" really is.

**What this doc gives you:** the layer underneath everything you already know. Source code → CPU instructions → binary file → OS loads it → process in RAM → talks to other processes → embeds a browser engine inside a window → ships as a `.app` or `.exe`. By the end you should know exactly what each of those words means and how they connect.

**How to read it:**
1. Linear, top to bottom. Each layer uses terms defined in the previous layer.
2. Every new term is **bold** the first time, and every bold term is in the glossary at Layer 1.
3. Every concept gets a **Backend analogy:** when one exists. Use those as your anchor.
4. Every section ends with **Verify it yourself:** — a read-only command you can run on your Mac to see the concept in real life. None of these change anything.

If a sentence has too many unfamiliar terms, scroll up to where each was first introduced. The doc is ~1500 lines exactly so you don't have to.

---

## Layer 1 — Glossary (skim now, return as needed)

### Hardware & runtime
| Term | Meaning |
|---|---|
| **CPU** | Central Processing Unit. The chip in your machine that executes instructions. M-series Macs have ARM64 CPUs; most PCs have x86_64. |
| **CPU instruction** | One operation the CPU knows: add two numbers, copy a byte, jump to address X. Few hundred to ~1500 of them total. |
| **Register** | A tiny, ultra-fast slot inside the CPU. M-series has ~32 general-purpose 64-bit registers. Used as scratch space during a calculation. |
| **RAM** | Random Access Memory. Volatile main memory. Holds running processes. Lost on power-off. |
| **MMU** | Memory Management Unit. A piece of the CPU that translates "virtual" addresses (what processes see) to physical RAM addresses. |
| **GPU** | Graphics Processing Unit. Specialized chip for parallel work like rendering pixels. |

### Code → executable
| Term | Meaning |
|---|---|
| **Source code** | Text you write (`.rs`, `.ts`, `.tsx`). The CPU cannot run this directly. |
| **Compiler** | Program that translates source code into machine code, ahead of time. `rustc`, `tsc`, `gcc`. |
| **Interpreter** | Program that reads source code and executes it line-by-line at runtime, no separate output file. Bash is a pure interpreter. |
| **JIT** | Just-In-Time compiler. Hybrid: starts as an interpreter, compiles hot paths to machine code while the program runs. V8 (Node, Chrome) does this. |
| **Machine code** | The CPU's native language: bytes that decode to instructions. Not human-readable. |
| **Assembly** | A 1-to-1 human-readable name for each machine instruction. `mov rax, 5` is assembly. |
| **IR** | Intermediate Representation. A format the compiler uses internally between "parsed source" and "machine code." Rust uses LLVM IR. |
| **Linker** | Tool that combines multiple compiled object files (`.o`) plus libraries into one binary. |
| **Static linking** | Copy the library's code into your binary at build time. Self-contained, bigger file. |
| **Dynamic linking** | Reference the library by name; OS loads it when your binary starts. Smaller file, but the library must be installed on the user's machine. |
| **Library** | Reusable compiled code. `libc` (the C standard library) is the most universal one. |
| **DLL** | Dynamic-Link Library. Windows's name for a dynamically-linked shared library. File ends in `.dll`. |
| **dylib** | macOS's name for the same thing. File ends in `.dylib`. |
| **.so file** | Linux's name for the same thing ("shared object"). |

### File formats
| Term | Meaning |
|---|---|
| **Binary** | Any file whose bytes are not human-readable text. Often used as shorthand for "executable file." |
| **Executable** | A binary the OS knows how to run as a process. |
| **Mach-O** | The macOS executable format. Files in `MyApp.app/Contents/MacOS/` are Mach-O. |
| **PE** | Portable Executable. Windows's format. `.exe` and `.dll` are both PE files. |
| **ELF** | Executable and Linkable Format. Linux's format. Most files in `/usr/bin/` on Linux are ELF. |
| **Header** | The first bytes of a binary, containing metadata: "I'm a Mach-O for ARM64, my entry point is at offset X." |
| **Segment** | A region of a binary with a single permission (read-only, read-write, executable). Loaded into memory as a unit. |
| **Section** | A subdivision of a segment. `.text` (code), `.data` (mutable globals), `.rodata` (constants), `.bss` (zero-init globals). |

### OS internals
| Term | Meaning |
|---|---|
| **Operating system** | The software that owns the machine and runs your programs. macOS, Windows, Linux. |
| **Kernel** | The innermost part of the OS. Has direct hardware access. Apps cannot — they ask the kernel for things. |
| **User space** | The world your apps live in. Restricted permissions. |
| **Kernel space** | The world the kernel lives in. Full hardware access. |
| **Syscall** | System Call. The function-shaped door from user space into kernel space. ~400 of them on macOS. |
| **OS loader** | The OS code that reads a binary file and turns it into a running process. macOS uses `dyld`; Windows uses the loader inside `ntdll.dll`. |
| **launchd** | macOS's "process #1." Starts everything else, including your app when you double-click it. |
| **LaunchServices** | macOS's database of "which app handles which file/URL/bundle ID." |
| **Gatekeeper** | macOS's pre-launch check: is this app code-signed and notarized? |
| **SmartScreen** | Windows's equivalent: is this `.exe` from a trusted publisher? |
| **UAC** | User Account Control. Windows's prompt asking "let this app run with admin rights?" |
| **Code signing** | Cryptographically proving "I (this developer) built this exact binary." Required to ship apps without scary warnings. |
| **Notarization** | Apple-specific extra step: Apple scans your signed binary for malware and gives you a ticket proving it's safe. |
| **Sandbox** | A box the OS draws around a process restricting what it can read, write, or call. WebView helper processes are sandboxed. |
| **Entitlement** | A specific permission a sandboxed app can be granted (e.g., "can use the camera"). |
| **Capability** | Tauri's term for the same idea, applied to which `#[tauri::command]` functions the frontend can invoke. |

### Process model
| Term | Meaning |
|---|---|
| **Process** | A running instance of a binary. Has its own memory, its own PID, its own file handles. |
| **PID** | Process ID. A unique integer the OS assigns when a process starts. |
| **Virtual memory** | The illusion that each process has the entire machine's RAM to itself. The MMU maintains the illusion. |
| **Address space** | The set of memory addresses one process can use. On 64-bit, 256 TB conceptually, far more than physical RAM. |
| **Page** | The unit of memory the OS hands out. Usually 4 KB or 16 KB. |
| **Stack** | A region of a process's memory used for function calls and local variables. Grows down. Bounded (~8 MB by default). |
| **Heap** | A region of a process's memory used for dynamic allocations (`Box::new`, `malloc`). Grows up. |
| **Thread** | A unit of execution inside one process. All threads in a process share its heap. |
| **Context switch** | When the CPU saves one thread's state and resumes another's. Happens thousands of times per second. |
| **Scheduler** | The kernel component that decides which thread runs on which CPU core right now. |
| **Preemption** | The scheduler interrupting a running thread to give the CPU to another one, even if the first one didn't want to yield. |
| **File descriptor** | A small integer the kernel hands you when you open a file/socket/pipe. You pass it back to read/write. Stdout is FD 1. |
| **Environment variable** | A key-value string inherited from the parent process. `process.env.NODE_ENV` in Node. |

### IPC family
| Term | Meaning |
|---|---|
| **IPC** | Inter-Process Communication. Any way two processes exchange data. |
| **Pipe** | A one-way byte stream between two processes. `ls \| grep foo` uses a pipe. |
| **Socket** | A two-way endpoint for communication. TCP sockets cross machines; Unix sockets stay on one machine. |
| **Unix domain socket** | A socket addressed by filesystem path instead of IP+port. Bypasses the network stack. |
| **Shared memory** | Both processes map the same physical RAM into their own address spaces. Fastest IPC. |
| **Message passing** | OS-mediated mailbox where one process sends a message and another receives it. |
| **Mach port** | macOS's primary message-passing primitive. Your app already uses thousands of them without knowing. |
| **XPC** | Apple's higher-level service IPC, built on Mach ports. WebKit's helper processes use XPC. |
| **Mojo** | Chromium's IPC system. Used by Chrome, Edge, and WebView2 to talk between renderer and browser processes. |

### Browser / WebView family
| Term | Meaning |
|---|---|
| **Browser** | The user-facing app: Safari, Chrome, Firefox. |
| **Browser engine** | The core machinery inside a browser: parses HTML, computes layout, paints pixels, runs JS. |
| **Rendering engine** | Often used interchangeably with "browser engine" but specifically refers to the layout + paint parts. |
| **Layout engine** | The part that decides where each box goes on screen. Subset of the rendering engine. |
| **JavaScript engine** | The part that executes JS. Separate from the HTML/CSS engine. |
| **V8** | Google's JavaScript engine. Inside Chrome and Node.js. |
| **JavaScriptCore** (JSC) | Apple's JavaScript engine. Inside Safari and WKWebView. |
| **SpiderMonkey** | Mozilla's JavaScript engine. Inside Firefox. Not relevant for our WebViews. |
| **WebKit** | Apple's open-source browser engine. Inside Safari, all iOS browsers (Apple's rule), and WKWebView. |
| **Blink** | Google's fork of WebKit, made in 2013. Powers Chrome, Edge, Brave, Opera. |
| **Chromium** | The open-source browser project that contains Blink + V8 + the chrome around them. "Chrome minus Google branding." |
| **WebView** | A reusable OS component that gives an app a browser engine inside its own window — no URL bar, no tabs. |
| **WKWebView** | macOS's WebView. Wraps WebKit + JavaScriptCore. |
| **WebView2** | Windows's WebView. Wraps Edge's Chromium + V8. |
| **WebKitGTK** | Linux's WebView. Wraps WebKit. |
| **DOM** | Document Object Model. The tree of HTML elements the browser builds. JS manipulates the DOM. |
| **Compositor** | The part of the engine that combines layers and pushes the final image to the GPU. |

### App shell
| Term | Meaning |
|---|---|
| **Native window** | An OS-managed window with the platform's titlebar, close/minimize buttons, etc. |
| **NSWindow** | macOS's class representing a native window. Provided by AppKit. |
| **HWND** | Windows's handle (a numeric ID) representing a native window. |
| **Event loop** | The infinite loop at the heart of every UI app: wait for an event (click, keypress, timer), dispatch it, repeat. |
| **.app bundle** | A macOS app: a directory ending in `.app` that Finder treats as one icon. |
| **Info.plist** | XML metadata file inside an `.app` bundle: bundle ID, version, required permissions, etc. |
| **Registry** | Windows's hierarchical key-value config database. App settings, install info, file associations. |
| **%APPDATA%** | Windows's per-user app data folder. Resolves to `C:\Users\You\AppData\Roaming`. |
| **~/Library/Application Support** | macOS's per-user app data folder. |
| **AppKit** | The macOS framework for building native UIs (windows, buttons, menus). |
| **Win32** | The Windows native API. Older, lower-level. |
| **Cocoa** | An umbrella name for macOS's native frameworks (AppKit + Foundation). |

### Stack we use in ZenC
| Term | Meaning |
|---|---|
| **Tauri** | Rust framework that gives you `OS WebView + native window + Rust process`, all in one binary. |
| **Electron** | Older framework that ships a full Chromium + Node.js inside your app. Heavier. |
| **Rust** | Systems language we write the backend in. Compiles to native machine code. |
| **Cargo** | Rust's package manager and build tool. |
| **npm / pnpm** | Node's package managers. We use one for the frontend. |
| **Vite** | Frontend build tool. Bundles `.tsx` to `bundle.js`. |
| **React** | UI library. Renders inside the WebView. |
| **TypeScript** | JavaScript with types. Compiles to JS for the WebView to run. |

OK. Now we build from the bottom up.

---

## Layer 2 — Source code becomes machine code

### 2.1 What a CPU actually does

A **CPU** is a chip that does one thing in a tight loop: fetch an instruction from memory, decode it, execute it, repeat. Modern CPUs do this ~3 billion times per second per core, and your laptop has 4–10 cores.

The CPU doesn't know about strings, JSON, HTTP, your `User` class, or React. It knows a fixed alphabet of ~1000–1500 **CPU instructions**. Examples:

- `mov rax, 5` — put the number 5 into register `rax`.
- `add rax, rbx` — add the value in `rbx` to `rax`, store the result in `rax`.
- `cmp rax, 10` — compare `rax` to 10, set flags.
- `jne some_label` — if the last comparison wasn't equal, jump to a different code address.
- `call my_function` — push the return address and jump to a function.
- `ret` — return from a function.

A **register** is one of ~32 tiny named slots inside the CPU. Each register holds 64 bits (8 bytes). All real work happens in registers — even adding two numbers from RAM means: load A into a register, load B into another, add, store back.

**Backend analogy:** if NestJS is "request comes in → handler runs → response goes out," the CPU is "instruction comes in → execute → next." The CPU is your most primitive request-response loop. The instructions are its only "endpoints."

### 2.2 Machine code is just bytes

When the CPU "executes" `mov rax, 5`, it's not reading the words `mov rax, 5`. Those words are an **assembly** representation for humans. The CPU reads these 7 bytes:

```
48 c7 c0 05 00 00 00
```

Each byte (or sequence of bytes) is a known pattern the CPU's silicon decodes into "this is `mov`, the destination is register `rax`, the value is the 32-bit number 5." Different instructions take different numbers of bytes (1 to ~15).

**Machine code** is just a long sequence of those bytes. A binary's `.text` section is mostly machine code: the encoded form of every function in your program.

**Backend analogy:** if assembly is `POST /add HTTP/1.1\nContent-Type: application/json\n\n{"a":2,"b":3}`, machine code is the actual TCP packet bytes the kernel sends — the same information, but in the wire format the receiver expects. The CPU is the receiver, and machine code is the wire format.

### 2.3 Source code is what you actually write

You don't write `mov rax, 5`. You write:

```rust
let x: u64 = 5;          // Rust
```

```typescript
const x: number = 5;     // TypeScript
```

That's **source code** — text you and your teammates can read. The CPU cannot run it directly. Something must turn it into machine code first.

There are three ways:

#### Compiler (Rust, C, Go, Swift)

A **compiler** reads your source code and produces a binary file containing machine code. The output runs without the compiler being present.

```
hello.rs   →  rustc  →  hello (Mach-O binary, 1 MB)
```

Once `hello` exists, you ship it. Users don't need `rustc` installed.

#### Interpreter (Bash, classic Python)

An **interpreter** reads your source code line-by-line and executes each line as it goes. There's no separate output file. Users need the interpreter installed to run your program.

```
script.sh   →  bash reads & executes line-by-line
```

#### JIT (V8 / Node, Java's JVM, Safari's JSC)

A **JIT** is a hybrid. It starts as an interpreter (fast startup, no compile delay) and watches which functions are called often. For "hot" functions, it secretly compiles them to machine code on the fly and substitutes the compiled version. So your `for` loop the first time runs interpreted, but if it runs a million times, V8 has long since recompiled it to native code.

```
server.ts   →  tsc  →  server.js  →  Node (V8)  →  starts interpreted, JITs hot paths
```

This is why Node feels fast even though JS is "interpreted." V8 is doing aggressive runtime compilation in the background.

### 2.4 Why this matters for Tauri

ZenC is going to ship one binary. Inside that binary, two execution models live side-by-side:

```
   ZenC.app/Contents/MacOS/zen-c (Mach-O binary, ~12 MB)
   ┌──────────────────────────────────────────────┐
   │  Native machine code (compiled by rustc)      │
   │  ────────────────────────────────────────────  │
   │  fn main() { ... tauri::Builder ... }          │
   │  fn start_focus_session(...) { ... }           │
   │  All Rust code as ARM64 instructions.          │
   ├──────────────────────────────────────────────┤
   │  Embedded resources (just bytes in .rodata)    │
   │  ────────────────────────────────────────────  │
   │  index.html (text)                             │
   │  bundle.js (JS source — JITed at runtime!)     │
   │  styles.css (text)                             │
   └──────────────────────────────────────────────┘
```

When ZenC starts, the Rust part is *already* machine code — the CPU runs it directly. The `bundle.js` is just text — it's handed to JavaScriptCore inside the WebView, which interprets it and JITs the hot paths.

So one process is doing both at once: Rust running as native code, React running as JIT-compiled JavaScript.

**Verify it yourself:** when you have a Rust binary built (later in the project), run:
```bash
file target/debug/zen-c
# → Mach-O 64-bit executable arm64
```
That single line confirms it's a binary, not source.

---

## Layer 3 — A binary file

### 3.1 What "binary" really means

The word **binary** is used two ways in tech:
1. As an adjective: "binary data" = bytes that aren't human-readable text.
2. As a noun: "the binary" = an executable file.

Both come from the same root: the file's contents are bytes that mean something specific to a machine, not letters meant for humans.

An **executable** is a binary the OS knows how to run as a process. Not every binary is executable: a `.png` is binary but not executable. A `.exe` is both.

### 3.2 Why each OS has its own format

The OS has to know how to read the binary file and copy its parts into memory in the right way. macOS, Windows, and Linux each invented their own format because the requirements were slightly different.

| OS | Format | File extension | Magic first bytes |
|---|---|---|---|
| **macOS** (and iOS) | **Mach-O** | none, or `.dylib` | `cf fa ed fe` (or its reverse) |
| **Windows** | **PE** (Portable Executable) | `.exe`, `.dll` | `4d 5a` (`MZ` in ASCII) |
| **Linux** | **ELF** (Executable and Linkable Format) | none, or `.so` | `7f 45 4c 46` (`\x7fELF`) |

If you copy a Windows `.exe` to a Mac and try to run it, macOS reads the first bytes, sees `4d 5a` instead of `cf fa ed fe`, and refuses. It's not a security check — it literally doesn't have the code to read PE files.

This is why building Rust for multiple OSes requires **cross-compilation**: you have to ask `rustc` to emit a different binary format for each target.

### 3.3 What's inside a binary (Mach-O specifically)

Here's the layout of a typical macOS binary:

```
   ┌──────────────────────────────────────────┐
   │ Mach-O Header                              │
   │   magic: cf fa ed fe                       │  "I'm a Mach-O 64-bit"
   │   cputype: ARM64                           │
   │   filetype: MH_EXECUTE                     │
   │   ncmds: 18                                │  18 load commands follow
   ├──────────────────────────────────────────┤
   │ Load Commands (the OS loader's TODO list)  │
   │   LC_SEGMENT_64 __TEXT                     │  "map __TEXT into memory, RX"
   │   LC_SEGMENT_64 __DATA                     │  "map __DATA into memory, RW"
   │   LC_LOAD_DYLIB /usr/lib/libSystem.B.dylib │  "load libc dynamically"
   │   LC_LOAD_DYLIB /System/.../WebKit         │  "load WebKit dynamically"
   │   LC_MAIN  entry_offset: 0x100003fa0       │  "start running here"
   │   ...                                       │
   ├──────────────────────────────────────────┤
   │ __TEXT segment  (Read-only, Executable)    │
   │   ┌────────────────────────────────────┐   │
   │   │ section __text                      │   │  ← all your fn bodies
   │   │   (machine code bytes)              │   │
   │   └────────────────────────────────────┘   │
   │   ┌────────────────────────────────────┐   │
   │   │ section __cstring                   │   │  ← string literals
   │   │   "Hello, world!\0"                 │   │
   │   │   "Failed to start session\0"        │   │
   │   └────────────────────────────────────┘   │
   │   ┌────────────────────────────────────┐   │
   │   │ section __const                     │   │  ← const data
   │   └────────────────────────────────────┘   │
   ├──────────────────────────────────────────┤
   │ __DATA segment  (Read-Write)               │
   │   ┌────────────────────────────────────┐   │
   │   │ section __data                      │   │  ← initialized globals
   │   └────────────────────────────────────┘   │
   │   ┌────────────────────────────────────┐   │
   │   │ section __bss                       │   │  ← zero-init globals
   │   └────────────────────────────────────┘   │
   ├──────────────────────────────────────────┤
   │ __LINKEDIT segment                          │
   │   symbol table, string table, signature     │
   └──────────────────────────────────────────┘
```

A **header** is just the first few bytes of the file telling the OS "what am I." A **segment** is a region with one set of permissions. A **section** is a finer-grained subdivision inside a segment with a specific purpose.

The four sections you'll hear about most:

| Section | Inside | Permissions | Purpose |
|---|---|---|---|
| `.text` (or `__text`) | Machine code bytes | Read + Execute | Your compiled functions live here |
| `.data` | Initialized globals/statics | Read + Write | `static FOO: i32 = 42;` lives here |
| `.rodata` (or `__cstring`/`__const`) | Constants and string literals | Read-only | `"hello"` literal lives here |
| `.bss` | Zero-initialized globals | Read + Write (lazy) | `static mut COUNTER: u32 = 0;` lives here |

When Tauri "embeds" your `index.html` and `bundle.js` in the binary, those file contents end up in `.rodata` — read-only constants the Rust code can pull bytes from at runtime.

### 3.4 Static vs dynamic linking

Your Rust code calls into other code: the standard library, third-party crates, the OS C library. There are two ways to bundle that:

```
   STATIC LINKING                          DYNAMIC LINKING
   ┌────────────────────────┐              ┌────────────────────────┐
   │ zen-c (Mach-O, 8 MB)    │              │ zen-c (Mach-O, 500 KB)  │
   │   __TEXT                │              │   __TEXT                │
   │     your code            │              │     your code            │
   │     std library code     │              │   __LINKEDIT             │
   │     serde code           │              │     LC_LOAD_DYLIB:       │
   │     tokio code           │              │       libSystem.B.dylib  │
   │     ...                  │              │       libstd.dylib       │
   └────────────────────────┘              └────────────────────────┘
   Self-contained.                          Smaller, but at runtime
   Runs anywhere of that OS.                the OS loader must find
                                            libSystem.B.dylib etc.
```

By default, **Rust statically links** other Rust crates (everything ends up in your binary), but **dynamically links** the OS C library (`libSystem` on macOS, `kernel32.dll` + `ucrtbase.dll` on Windows). This is why a Rust binary still depends on the OS — it's not "fully self-contained."

The trade-off:
- **Static** = bigger binary, no missing-dependency errors.
- **Dynamic** = smaller binary, the OS library can be patched without rebuilding your app, but if the user is missing the right version you're stuck.

For ZenC, you don't need to think about this — Tauri's defaults are fine.

**Verify it yourself:** on macOS, once you have any binary (say `/bin/ls`):
```bash
file /bin/ls               # tells you the format
otool -h /bin/ls           # prints the header
otool -L /bin/ls           # lists every dylib it depends on
otool -l /bin/ls | head    # prints load commands (verbose)
```
When you eventually have `target/debug/zen-c`, run the same commands and you'll see your dependencies (libSystem, WebKit, etc.).

---

## Layer 4 — Operating system fundamentals

Before processes, you need to know what an OS actually is.

### 4.1 What an OS does

An **operating system** is software that manages the machine's hardware (CPU time, RAM, disk, network, screen, keyboard) and runs other programs on top. macOS, Windows, and Linux are operating systems.

Roughly, the OS:
1. Owns all hardware. Decides who gets the CPU, who gets RAM, who can read what file.
2. Provides a **process** abstraction so programs don't trample each other.
3. Provides a stable API (system calls) so programs don't have to know hardware details.
4. Provides drivers so apps can use the screen, mouse, network, USB, etc.

**Backend analogy:** the OS is to your app what Kubernetes is to a pod. Kubernetes decides which node a pod runs on, gives it isolated networking and storage, restarts it if it dies, and keeps multiple pods on one node from stepping on each other. The OS does the same for processes on one machine.

### 4.2 Kernel vs user space

Inside the OS there's a privileged inner core called the **kernel**. The kernel has direct hardware access — it can talk to the disk controller, configure the MMU, change CPU privilege levels.

Everything else — including your Rust app, Chrome, Slack, the macOS Finder, even most of macOS itself — runs in **user space** with restricted permissions. User-space code cannot directly access disk hardware, write to arbitrary memory, or stop other processes.

```
   ┌──────────────────────────────────────────────────────┐
   │  USER SPACE                                            │
   │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌────────┐ │
   │  │ ZenC      │  │ Chrome    │  │ Slack     │  │ Finder │ │
   │  │ (process) │  │ (process) │  │ (process) │  │        │ │
   │  └─────┬────┘  └─────┬────┘  └─────┬────┘  └───┬────┘ │
   │        │             │             │            │       │
   │        │   syscalls (open, read, write, ...)            │
   │        ▼             ▼             ▼            ▼       │
   ├──────────────────────────────────────────────────────┤
   │  KERNEL SPACE                                          │
   │  ┌────────────────────────────────────────────────┐   │
   │  │ XNU kernel (macOS) / NT kernel (Windows)        │   │
   │  │   - process scheduler                           │   │
   │  │   - virtual memory manager                       │   │
   │  │   - filesystem                                   │   │
   │  │   - network stack                                │   │
   │  │   - device drivers                               │   │
   │  └────────────────────────────────────────────────┘   │
   ├──────────────────────────────────────────────────────┤
   │  HARDWARE                                              │
   │  CPU · RAM · disk · network card · GPU · keyboard      │
   └──────────────────────────────────────────────────────┘
```

**Backend analogy:** user space is your application code; kernel space is the database engine. You don't poke directly at Postgres's storage files — you send SQL through a connection. Apps don't poke directly at disk sectors — they call `read()` and the kernel does it.

### 4.3 Syscalls — the kernel's API

A **syscall** (system call) is the function-shaped doorway from user space into the kernel. When your Rust code does:

```rust
let mut file = std::fs::File::open("hello.txt")?;
```

…the chain looks like:

```
   Your Rust code
        │
        │ calls
        ▼
   std::fs::File::open
        │
        │ ultimately calls
        ▼
   libc's open() function (linked dynamically from libSystem.B.dylib)
        │
        │ executes a special CPU instruction (svc on ARM64) that
        │ switches to kernel mode
        ▼
   ┌─────────────────────────────┐
   │  KERNEL: open syscall        │
   │   - check permissions        │
   │   - find file on disk        │
   │   - create file descriptor   │
   │   - return FD to user space  │
   └─────────────────────────────┘
        │
        │ returns
        ▼
   Your code now has a File handle
```

That's it. Every "interesting" thing your app does — read a file, open a socket, kill a process, allocate memory pages, look up the time — ultimately becomes one or more syscalls. There are about 400 syscalls on macOS and ~1000 on Windows.

**Why this matters for ZenC:** when ZenC blocks an app, it'll do so via specific syscalls the OS exposes (`kill` to terminate, screen-time APIs to enforce time limits, accessibility APIs to monitor what's running, etc.). The OS decides whether to allow each one based on permissions you've requested in `Info.plist`.

**Verify it yourself:** on macOS, you can trace syscalls of any command:
```bash
sudo dtruss ls 2>&1 | head -20
# (you'll see open, read, mmap, write, exit syscalls scrolling by)
```

---

## Layer 5 — Processes & memory

This is the longest layer. Take your time. Everything later builds on this.

### 5.1 What a process really is

A **process** is a running instance of a binary. The binary is a passive file on disk. The process is alive — it's loaded into RAM, has an ID, and is currently being scheduled by the kernel.

When you run `node server.js`:
- The binary `node` is on disk at `/usr/local/bin/node`.
- The process is created when you run the command. It gets a PID.
- Running it again creates a *second* process from the *same* binary.

```
   On disk:                         In RAM:
   ┌────────────────┐               ┌─────────────────┐
   │ /usr/local/bin │               │ PID 14821:       │
   │   /node         │  ───────►   │   node server.js │
   │   (15 MB file)  │   spawn      │   (uses ~80 MB)   │
   └────────────────┘               └─────────────────┘
                                    ┌─────────────────┐
                                    │ PID 14822:       │
                                    │   node script.js │
                                    │   (uses ~50 MB)   │
                                    └─────────────────┘
```

Same `node` binary, two independent processes.

A **PID** (process ID) is the unique integer the OS gives each process. Run `ps -ef` (or `ps aux`) on your Mac to see them.

**Backend analogy:** the binary is a Docker image (passive, on disk). The process is a running container (alive, has its own ID, can be stopped). Two `docker run` commands of the same image = two containers. Two `node server.js` invocations = two processes.

### 5.2 What a process owns

Every process has its own private set of resources the OS hands it:

| Resource | Description | NestJS analogue |
|---|---|---|
| **PID** | Unique integer ID | Container ID |
| **Address space** | Its own private virtual memory map | Container's filesystem namespace |
| **File descriptors** | Open files, sockets, pipes (each a small int) | Open DB connection / HTTP client handles |
| **Threads** | One or more units of execution inside | Worker threads |
| **Environment variables** | Key/value config inherited from parent | `process.env` |
| **Working directory** | Where relative paths resolve | `process.cwd()` |
| **User/group ID** | Whose privileges does it run with | Container's runAs user |
| **Open ports** | TCP/UDP sockets bound to ports | listening sockets |

When the process exits, the kernel reclaims everything: closes the FDs, frees the memory, removes the PID. There's no leak unless the OS itself is buggy.

### 5.3 Virtual memory — the big illusion

Every process thinks it has the entire RAM of the machine to itself. That's the **virtual memory** illusion. The truth is much weirder.

```
   Process A's view (its address space)        Process B's view (its address space)
   ┌─────────────────────┐                     ┌─────────────────────┐
   │ 0xFFFFFFFFFFFFFFFF   │                     │ 0xFFFFFFFFFFFFFFFF   │
   │   stack              │                     │   stack              │
   │   ↓                  │                     │   ↓                  │
   │                      │                     │                      │
   │   ↑                  │                     │   ↑                  │
   │   heap               │                     │   heap               │
   │   .data              │                     │   .data              │
   │   .text              │                     │   .text              │
   │ 0x0000000000000000   │                     │ 0x0000000000000000   │
   └─────────────────────┘                     └─────────────────────┘
              │                                            │
              │  Both think they own address 0x100000000.  │
              ▼                                            ▼
   ┌────────────────────────────────────────────────────────────┐
   │   Memory Management Unit (part of the CPU)                  │
   │   Translates each process's "virtual" address               │
   │   to a "physical" RAM address using a per-process page table │
   └────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
   ┌────────────────────────────────────────────────────────────┐
   │   Physical RAM (shared across the whole machine)            │
   │     Process A's "0x100000000" → physical 0x7AF30000          │
   │     Process B's "0x100000000" → physical 0x4C210000          │
   └────────────────────────────────────────────────────────────┘
```

The kernel maintains a **page table** for each process: "address X → physical RAM page Y, address X+4096 → physical RAM page Z, ..." The MMU consults this table millions of times per second.

Two consequences:
1. **Isolation.** Process A cannot read or corrupt Process B's memory. The MMU literally won't let address X in Process A reach the same physical page as address X in Process B.
2. **Overcommit.** The total virtual memory across all processes can be way larger than physical RAM. The kernel pages unused parts to disk.

**Backend analogy:** Docker namespaces give each container its own `/`, its own network interfaces, its own PIDs. Two containers can both have `/etc/passwd` and not see each other's. Virtual memory is the same idea applied to RAM: every process gets its own "address space" with no idea the others exist.

### 5.4 What the memory of one process looks like

Inside a single process's address space, memory is laid out roughly:

```
   High addresses (0xFFFFFFFFFFFFFFFF)
   ┌───────────────────────────────┐
   │ Stack                          │  ← grows DOWN
   │  ↓                             │     - Function call frames live here.
   │                                │     - Local variables: `let x = 5;` puts x here.
   │     (free space)               │     - Bounded — usually 8 MB. If you exceed it:
   │                                │       "stack overflow" (e.g., infinite recursion).
   │  ↑                             │
   │ Heap                           │  ← grows UP
   │                                │     - Dynamic allocations: Box, Vec, String.
   │                                │     - You ask the allocator (which asks the kernel).
   ├───────────────────────────────┤
   │ .bss   (zero-initialized)      │  - `static mut COUNTER: u32 = 0;` lives here
   │ .data  (initialized globals)   │  - `static NAME: &str = "Zen";` partially here
   │ .rodata (constants, literals)  │  - `"Hello"` string literal, embedded HTML/JS
   │ .text  (your machine code)     │  - `fn main()` and every other compiled function
   ├───────────────────────────────┤
   │ (reserved by the kernel)        │
   └───────────────────────────────┘
   Low addresses (0x0000000000000000)
```

The bottom four sections (`.text`, `.rodata`, `.data`, `.bss`) come straight from the binary file — the OS loader copies them in.

The **stack** is preallocated for each thread. Every function call pushes a frame (with the function's local variables and return address) onto it; every return pops one off. It's a tower that grows and shrinks as you call/return.

The **heap** is what `malloc`, `Box::new`, `Vec::push`, `String::from`, etc. ultimately use. The Rust allocator (or Node's V8 allocator) maintains this region and asks the kernel for more pages when it runs low.

### 5.5 Stack vs heap — concrete Rust examples

```rust
fn make_session() -> Session {
    let id: u64 = 42;                    // STACK: id is a 64-bit number on the stack
    let name = "Deep Work";              // STACK: pointer & length on stack;
                                         //        the bytes "Deep Work" are in .rodata
    let participants: Vec<String> = vec![
        String::from("Harish"),          // HEAP: each String's bytes are heap-allocated;
        String::from("Bot"),             //       the Vec's buffer is heap-allocated too.
    ];                                   // STACK: the Vec struct (3 words: ptr/len/cap)
                                         //        is on the stack.
    let session = Session {              // STACK: the Session struct is on the stack
        id,
        name,
        participants,
    };
    session                              // ← when returned, contents are MOVED to caller's
}                                        //   stack frame; heap allocations stay on the heap.
```

Rough rule:
- Fixed-size things (numbers, structs of numbers, pointers) → **stack**.
- Things that grow at runtime (`Vec`, `String`, `Box<T>`, `HashMap`) → the **container struct** is on the stack, but it owns a buffer on the **heap**.

A **stack overflow** happens when the stack grows beyond its limit — typically infinite recursion:

```rust
fn boom() { boom(); }   // each call pushes a new frame; stack runs out in ~1M calls
```

You'll see "stack overflow" in the terminal and the process is killed.

### 5.6 Threads vs processes

A **thread** is a unit of execution inside one process. All threads in a process **share the heap** and all global state. They each have their own stack (so each thread can call functions independently).

```
   ONE PROCESS                                 TWO PROCESSES
   ┌──────────────────────────────┐            ┌──────────────┐  ┌──────────────┐
   │ Process (PID 14821)           │            │ Process A     │  │ Process B     │
   │  ┌─────────┐  ┌─────────┐     │            │  ┌─────────┐  │  │  ┌─────────┐  │
   │  │ Thread 1 │  │ Thread 2 │   │            │  │ Thread   │  │  │  │ Thread   │  │
   │  │ stack    │  │ stack    │   │            │  │ stack    │  │  │  │ stack    │  │
   │  └─────────┘  └─────────┘     │            │  └─────────┘  │  │  └─────────┘  │
   │  ─────── shared heap ───────  │            │   own heap    │  │   own heap    │
   │  ─────── shared .data ──────  │            │   own .data   │  │   own .data   │
   └──────────────────────────────┘            └──────────────┘  └──────────────┘
   Cheap to start (~µs).                       Expensive to start (~ms).
   Crash takes everything down.                Crash isolated to one.
   Shared memory (mutexes needed).             Must use IPC to talk.
```

Node is single-threaded for *your* JavaScript, but uses background threads internally (libuv) for I/O. Rust gives you real OS threads via `std::thread::spawn`, plus async tasks via Tokio (cheaper than threads, multiplexed onto a few worker threads).

In Tauri, the UI thread (running the WebView event loop) and the Rust backend run as **threads in the same process** — that's why their IPC bridge can be fast. (Caveat: as we'll see in Layer 7, the WebView itself spawns *other* helper processes that run the actual web rendering — so it's both at once.)

### 5.7 The scheduler — how 1000 processes share 8 cores

Your Mac has 8 (or so) CPU cores. It also has hundreds of running processes at any moment. How?

The kernel **scheduler** time-slices: every few milliseconds, it forcibly interrupts the running thread, saves its CPU registers and state (a **context switch**), and resumes a different thread on that core. This is **preemption** — the thread didn't choose to yield; the kernel made it.

```
   Time →
   Core 0: [chrome ][zen-c  ][kernel][chrome ][slack ][zen-c ][kernel]...
   Core 1: [vscode ][slack  ][zen-c ][vscode ][kernel][...
   Core 2: [...
   Core 3: [...
            ↑ each block ≈ 1–10 ms
```

Each block above is one thread getting a slice of CPU time. The kernel decides who runs based on priorities, niceness, and "who has been waiting longest."

This is why "is your app fast?" really means "does your app yield the CPU often, and use little of it when it does run?" Tauri's advantage over Electron here is real: idle Tauri apps use almost no CPU because Rust isn't doing anything; idle Electron apps still have V8 GC and Chromium internals running.

**Verify it yourself:** open Activity Monitor (or `top`) on your Mac. Sort by %CPU. You'll see hundreds of processes, but most are at ~0% — they're not running, they're parked, waiting for an event.

---

## Layer 6 — How processes talk to each other (IPC)

OK. Each process has isolated memory. But programs need to talk: NestJS talks to Postgres, your browser talks to a server, Slack helper talks to Slack main. How?

### 6.1 IPC — the umbrella term

**IPC** stands for **Inter-Process Communication**. It's the umbrella term for every technique two processes use to exchange data. The OS provides a menu; programs pick from it.

You already use IPC every day:
- React talks to NestJS over **HTTP**. (IPC across machines.)
- NestJS talks to PostgreSQL over a **TCP socket** with the libpq protocol. (IPC, usually across machines.)
- NestJS talks to Redis over a **TCP socket** with the RESP protocol. (Same.)
- `ls | grep foo` is `ls`'s output going through a **pipe** into `grep`'s input.
- Spawning a child process and reading its stdout is a pipe.

All forms of IPC. The differences are in latency, throughput, addressing, and what kinds of messages they carry.

### 6.2 The IPC menu

| Mechanism | One-line description | Used for |
|---|---|---|
| **TCP socket** | Two-way byte stream addressed by IP+port. Crosses machines. | HTTP, Postgres, Redis. |
| **Unix domain socket** | Two-way byte stream addressed by filesystem path. Same machine only. Bypasses the network stack — faster. | Postgres local clients, Docker daemon socket. |
| **Pipe** | One-way byte stream between two processes. Usually parent-child. | Shell pipelines, `Command::output()`. |
| **Named pipe (FIFO)** | A pipe addressable by filesystem path. | Old-school IPC; rare today. |
| **Shared memory** | Both processes map the same physical RAM into their own address spaces. Fastest. | Real-time audio, video, GPU work. Hardest to use safely. |
| **Message passing** | The OS mediates a typed mailbox: send a message, receive a message. | Foundation for higher-level IPC on every OS. |
| **Mach ports (macOS)** | macOS's primary message-passing primitive. | XPC, Cocoa events, WKWebView↔app talk. |
| **Mojo (Chromium)** | Chromium's high-level message passing across processes. | All Chrome / Edge / WebView2 internal IPC. |
| **D-Bus (Linux)** | Desktop bus for app-to-app messaging. | Notifications, system tray on Linux. |

You don't need to memorize this. Just know: IPC is a menu. HTTP is one item; there are many faster, more local options.

### 6.3 In-process function calls aren't IPC

If two pieces of code are in the **same process**, they can just call each other:

```rust
fn foo() { bar(); }       // foo and bar share memory.
                          // No IPC needed; this is just a function call.
```

The CPU pushes a frame onto the same stack and jumps. Nanoseconds. No serialization. No kernel involved.

This is the fast path Tauri tries to maximize. Most "Tauri commands" feel like function calls because most of the data flow is one process. But...

### 6.4 The Tauri twist — WebView is a separate process

Here's the surprise. WKWebView and WebView2 are **multi-process** for security. The HTML/JS doesn't actually run in your `zen-c` process. It runs in a sandboxed helper process the OS spawns next to yours.

So when React calls `invoke('start_focus_session')`, that's actually:
- JavaScript runs in helper process A (sandboxed WebContent).
- The function `start_focus_session` runs in your `zen-c` process (the main one).
- They have different PIDs and different memory.
- They must use **IPC**.

Tauri picks the fastest local option each OS provides:
- **macOS:** Mach ports (via XPC under the hood).
- **Windows:** Mojo (via Chromium's IPC layer).

The result is microsecond latency, not millisecond, because there's no network stack, no JSON over HTTP, no socket buffer copies.

### 6.5 Anatomy of a single `invoke` call

```
  WebContent helper process                    Your zen-c main process
  (sandboxed, JavaScriptCore)                  (Rust)
  ┌───────────────────────────────┐            ┌────────────────────────────┐
  │ React calls:                   │            │  #[tauri::command]          │
  │   await invoke(                │            │  fn start_focus_session(    │
  │     'start_focus_session',     │            │      minutes: u32           │
  │     { minutes: 25 })           │            │  ) -> Result<Session,...> { │
  │       ↓                        │            │      ...                    │
  │ Tauri-injected JS calls        │            │  }                          │
  │   window.__TAURI_INTERNALS__   │            │                             │
  │     .invoke(...)                │            │  Tauri command router:      │
  │       ↓                        │            │   - matches command name    │
  │ Serializes args to JSON         │            │   - deserializes JSON args  │
  │       ↓                        │            │   - calls the fn            │
  │ Posts message via               │ ─────────► │   - serializes the return   │
  │   webkit.messageHandlers (mac) │  Mach port │   - sends back               │
  │   chrome.webview.postMessage   │  / Mojo    │                             │
  │       (Windows)                 │            │                             │
  │       ↓                        │ ◄───────── │                             │
  │ awaits the Promise              │            │                             │
  │       ↓                        │            │                             │
  │ Promise resolves with result    │            │                             │
  └───────────────────────────────┘            └────────────────────────────┘
```

Looks like a function call. Actually a JSON round-trip across process boundaries. But it's *local* — kernel-mediated message passing — so it's typically 10–100 microseconds, not the 1–10 ms of HTTP/loopback.

**Backend analogy:** if NestJS↔Postgres on localhost is "TCP IPC over loopback" (slowest local IPC, but easy and standard), Tauri's `invoke` is "Mach port message" (fastest available local IPC, hidden behind a function-shaped API). Same idea, much shorter pipeline.

**Verify it yourself:** open Activity Monitor while a Tauri or Electron app is running. Look for processes whose names include `WebKit.WebContent`, `WebKit.Networking`, `WebKit.GPU` (for WKWebView) or `msedgewebview2.exe` (for WebView2). Each is a separate process tied to the parent app — that's the IPC happening across them.

---

## Layer 7 — Inside a WebView

Now we have everything we need to explain WebViews properly.

### 7.1 What a browser actually is

When you open Safari and visit a page, three big systems work together:

1. **Layout/rendering engine** — parses HTML into a DOM tree, parses CSS into rules, computes where every box goes, paints pixels.
2. **JavaScript engine** — parses JS, JIT-compiles hot paths, runs your `useState`, `setTimeout`, fetch, etc. Manipulates the DOM.
3. **Compositor / GPU pipeline** — combines layers, hands the final frame to the GPU.

```
   ┌──────────────────────────────────────────┐
   │  HTML/CSS engine (e.g., WebKit, Blink)    │
   │  Parses HTML → DOM tree                    │
   │  Parses CSS → rules                        │
   │  Computes layout (box positions, sizes)    │
   │  Paints into a 2D buffer                   │
   └──────────────────────────────────────────┘
                  ▲
                  │ JS manipulates the DOM
                  ▼
   ┌──────────────────────────────────────────┐
   │  JavaScript engine (e.g., V8, JSC)        │
   │  Parses JS → bytecode                      │
   │  JITs hot paths to native code             │
   │  Garbage collects objects                  │
   │  Calls into the layout engine via DOM API  │
   └──────────────────────────────────────────┘
                  ▲
                  │ paints frames
                  ▼
   ┌──────────────────────────────────────────┐
   │  Compositor / GPU pipeline                 │
   │  Layers + GPU = smooth scroll/animate      │
   └──────────────────────────────────────────┘
```

A **WebView** is exactly these three components, repackaged so you can drop them inside a native window of your own app — without the URL bar, tabs, bookmarks, or any other browser chrome.

### 7.2 The browser engine family tree

Here are the big names. Knowing who built what helps everything else click.

```
   1998:  KHTML (KDE project, Linux)
            │
            ▼
   2003:  Apple forks KHTML → WebKit (used in Safari)
            │
            │  (2013)
            ▼
   2013:  Google forks WebKit → Blink (used in Chrome)
            │
            ▼
   2020:  Microsoft drops EdgeHTML, switches to Blink/Chromium
            │
            ▼
   Today: Chrome, Edge, Opera, Brave, Vivaldi → all Blink/Chromium
          Safari, all iOS browsers (mandated)  → WebKit
          Firefox                                → Gecko (separate lineage)
```

Translation:

| Engine name | Whose | Where it lives |
|---|---|---|
| **WebKit** | Apple | Safari, all iOS browsers, **WKWebView** |
| **Blink** | Google (forked from WebKit) | Inside **Chromium** |
| **Chromium** | Open-source project containing Blink + V8 + browser shell | Chrome, Edge, Brave, Opera, **WebView2** |
| **Gecko** | Mozilla | Firefox |

The JavaScript engines are separate but coupled:

| JS engine | Whose | Lives inside |
|---|---|---|
| **V8** | Google | Chromium / Node.js |
| **JavaScriptCore (JSC)** | Apple | WebKit |
| **SpiderMonkey** | Mozilla | Gecko |

### 7.3 The two WebViews ZenC actually uses

#### WKWebView (macOS)

```
   Your ZenC.app process (Rust + Tauri)
   ┌───────────────────────────────────────────────┐
   │  fn main() { tauri::Builder::default()... }    │
   │                                                │
   │  ┌─────────────────────┐                       │
   │  │  WKWebView host view │ ← inside your NSWindow│
   │  └──────────┬──────────┘                       │
   │             │                                   │
   └─────────────┼───────────────────────────────────┘
                 │
                 │ XPC (Mach-port-based IPC)
                 ▼
   ┌───────────────────────────────────────────────┐
   │ com.apple.WebKit.WebContent (separate PID!)    │
   │  ┌───────────────────────────────────────┐    │
   │  │  WebKit core                            │    │
   │  │  - HTML parser                           │    │
   │  │  - CSS layout                            │    │
   │  │  - JavaScriptCore (your React JS!)       │    │
   │  └───────────────────────────────────────┘    │
   │  Sandboxed: cannot read your filesystem.        │
   └───────────────────────────────────────────────┘

   ┌───────────────────────────────────────────────┐
   │ com.apple.WebKit.Networking (separate PID)      │
   │ All HTTP/HTTPS requests funnel through here.    │
   └───────────────────────────────────────────────┘

   ┌───────────────────────────────────────────────┐
   │ com.apple.WebKit.GPU (separate PID)             │
   │ Rasterization & GPU compositing.                │
   └───────────────────────────────────────────────┘
```

WKWebView in your app is just a "host" view — it forwards drawing and events. The actual web rendering happens in helper processes, sandboxed away. If a malicious page exploits a JS bug, it's stuck in WebContent's sandbox; it can't read your home directory.

WKWebView has shipped with macOS since 10.10 (2014). You don't install or ship it; it's already on every Mac running ZenC.

#### WebView2 (Windows)

```
   Your ZenC.exe process (Rust + Tauri)
   ┌───────────────────────────────────────────────┐
   │  WebView2 host control inside HWND             │
   └─────────────────────┬─────────────────────────┘
                         │
                         │ Mojo (Chromium IPC)
                         ▼
   msedgewebview2.exe (browser process)
   ┌───────────────────────────────────────────────┐
   │  Spawns:                                        │
   │  ┌──────────────┐  ┌──────────────┐            │
   │  │ Renderer     │  │ GPU process  │            │
   │  │ (V8 + Blink) │  │              │            │
   │  └──────────────┘  └──────────────┘            │
   │  Sandboxed.                                     │
   └───────────────────────────────────────────────┘
```

Same multi-process pattern, different vendor. WebView2 is auto-installed on Windows 10 (Edge updates ship it) and built into Windows 11.

### 7.4 Why Tauri renders slightly differently on each OS

Because the engines are different (WebKit on Mac, Blink on Windows), the same React app may show subtle differences:
- Slightly different default fonts.
- Slightly different scrollbar behaviour.
- Slightly different CSS edge-cases (newer features may land in one engine first).

For ZenC (a focus app, not a design tool), this is fine. Tauri's documentation has a list of "polyfill the engine difference" tricks if needed.

Electron sidesteps this by shipping its own copy of Chromium with the app — pixel-identical everywhere, but at the cost of ~150 MB and noticeably more RAM.

### 7.5 The native ↔ JS bridge

How does JavaScript inside the WebView call your Rust function? Tauri injects a small JS object into the page when it loads:

```js
window.__TAURI_INTERNALS__.invoke = function(cmd, payload) {
  // 1. Serialize payload to JSON.
  // 2. Tag it with a unique callback ID.
  // 3. Post a message to the native side via the OS-specific channel:
  //      - macOS: webkit.messageHandlers.tauri.postMessage(...)
  //      - Windows: window.chrome.webview.postMessage(...)
  // 4. Return a Promise that resolves when the native side replies.
};
```

The Rust side has set up handlers for those messages. When one arrives, Tauri's command router:
1. Reads the command name.
2. Looks up the matching `#[tauri::command]` function.
3. Deserializes the JSON payload into the function's parameter types.
4. Calls the function.
5. Serializes the return value (or error).
6. Sends it back to the JS side using the original callback ID.
7. The JS Promise resolves with the result.

That's the whole bridge. It looks like a function call from React's perspective. Under the hood it's local cross-process JSON message passing.

### 7.6 Tauri vs Electron — concrete trade-offs

| | **Electron** | **Tauri** |
|---|---|---|
| Browser engine | Bundled Chromium (~150 MB) | OS WebView (0 MB shipped) |
| JS engine | Bundled V8 | OS-provided (JSC on Mac, V8 on Windows via WebView2) |
| Backend | Node.js process (~80 MB more) | Rust in same binary |
| Bridge | `contextBridge` / `ipcMain.handle` | `invoke()` |
| Idle RAM | ~150-300 MB | ~30-80 MB |
| Cold start | ~2-3 s | ~50-200 ms |
| Cross-OS rendering | Identical (it's the same Chromium) | Slightly different per OS |
| Build complexity | Single Node toolchain | Rust + Node toolchains |

**Verify it yourself:** open Activity Monitor → search for `Slack`. You'll see one main process plus ~5 helper "Slack Helper" processes (each is a Chromium process). Then look for any Tauri app (or wait until ZenC runs) — you'll see one main, plus a few `WebKit.*` helpers. The architecture is roughly the same shape; only the bundled-vs-OS choice differs.

---

## Layer 8 — How macOS and Windows actually run an app

### 8.1 macOS — the `.app` bundle

A macOS "application" is actually a **directory** ending in `.app` that the Finder displays as a single icon. Right-click any `.app` → "Show Package Contents" to see inside.

```
   ZenC.app/                          ← what users see in the Dock
   └── Contents/
       ├── Info.plist                 ← XML metadata (more on this below)
       ├── MacOS/
       │   └── zen-c                  ← the actual Mach-O binary
       ├── Resources/
       │   ├── AppIcon.icns           ← the icon shown everywhere
       │   └── en.lproj/              ← localized strings
       ├── Frameworks/                ← any bundled .dylib files
       └── _CodeSignature/
           └── CodeResources          ← Apple's tamper-detection signatures
```

**Info.plist** is critical. It declares things like:

```xml
<key>CFBundleIdentifier</key>
<string>se.surfboard.zen-c</string>      <!-- unique app ID -->

<key>CFBundleVersion</key>
<string>0.1.0</string>

<key>NSCameraUsageDescription</key>
<string>ZenC needs the camera to ...</string>  <!-- required if you use camera -->

<key>LSMinimumSystemVersion</key>
<string>11.0</string>                          <!-- minimum macOS version -->

<key>LSUIElement</key>
<true/>                                         <!-- "I don't show in the Dock" -->
```

If you ask for camera/mic/photos/contacts/etc. without a usage description, the OS terminates your app. This is the macOS permission system. You'll edit `Info.plist` (or its `tauri.conf.json` source) every time you need a new permission.

#### macOS launch flow

```
   1. User double-clicks ZenC.app in Finder.
                ↓
   2. Finder asks LaunchServices: "what handles bundle ID se.surfboard.zen-c?"
                ↓
   3. LaunchServices reads Info.plist, finds Contents/MacOS/zen-c.
                ↓
   4. Gatekeeper checks:
        - Is the binary code-signed by a known Apple Developer ID?
        - Is it notarized? (Apple has scanned it for malware.)
        - If neither: "ZenC can't be opened, it's from an unidentified developer."
                ↓
   5. XProtect runs (built-in malware scan, quick).
                ↓
   6. posix_spawn() — kernel creates a new process.
        - dyld (the loader) reads the Mach-O.
        - dyld maps __TEXT and __DATA into virtual memory.
        - dyld loads dependencies (libSystem.B.dylib, WebKit, etc.).
        - dyld jumps to the entry point (LC_MAIN's offset).
                ↓
   7. Your fn main() runs.
        - tauri::Builder asks AppKit to create an NSApplication.
        - NSApplication starts the Cocoa event loop.
        - Tauri creates an NSWindow.
        - Tauri inserts a WKWebView into the window.
        - WKWebView spawns its WebContent / Networking / GPU helpers.
        - WebContent loads the embedded index.html.
        - JavaScriptCore evaluates bundle.js.
        - React mounts.
                ↓
   8. App is alive. Visible in the Dock.
                ↓
   9. User clicks the red X.
        - macOS convention: window closes but the app keeps running.
        - To fully quit: ⌘Q or right-click Dock icon → Quit.
                ↓
   10. App calls exit() / NSApplicationTerminate.
       Kernel reclaims memory, FDs, child processes (WebContent, etc.).
```

#### Code signing & notarization (you'll need both to ship)

Without code signing, macOS treats your app as "from an unidentified developer" and refuses to open it. The pipeline:

```
   Build:          cargo tauri build → ZenC.app
       ↓
   Sign:           codesign --sign "Developer ID: Surfboard" ZenC.app
       ↓           (proves Surfboard built it. Cert costs $99/yr.)
   Package:        Tauri produces a .dmg.
       ↓
   Notarize:       xcrun notarytool submit ZenC.dmg --wait
       ↓           (Apple scans it, returns ticket in ~5 min.)
   Staple:         xcrun stapler staple ZenC.app
       ↓           (embeds notarization ticket so offline launches work.)
   Distribute.
       ↓
   User downloads → Gatekeeper sees signed+notarized → opens silently.
```

Skip notarization → users see "ZenC.app is damaged and can't be opened." This catches every team the first time.

### 8.2 Windows — `.exe` and surroundings

Windows has no "bundle" concept. An app installs into a folder, typically under `C:\Program Files\YourApp\`.

```
   C:\Program Files\ZenC\
   ├── zen-c.exe                       ← the PE binary
   ├── WebView2Loader.dll              ← Tauri's WebView2 helper
   ├── resources/
   │   └── ... (frontend assets)
   └── uninstall.exe                   ← MSI uninstaller
```

There's no Info.plist; metadata is split between:
- The PE file's resource section (icon, version info).
- A manifest XML (UAC requirements, supported Windows versions).
- The Registry (file associations, install info, Start menu shortcut).

#### Windows launch flow

```
   1. User double-clicks zen-c.exe (or its Start menu shortcut).
                ↓
   2. Explorer.exe calls CreateProcess().
                ↓
   3. SmartScreen reputation check:
        - Is the EXE Authenticode-signed?
        - Is the publisher certificate trusted?
        - Has Microsoft seen this binary before?
        - Unknown → "Windows protected your PC" warning.
                ↓
   4. UAC (User Account Control):
        - Does the manifest request elevation?
        - If yes, prompt for admin password.
                ↓
   5. Windows loader (inside ntdll.dll):
        - Parses PE header.
        - Maps .text/.data into virtual memory.
        - Loads imported DLLs (kernel32.dll, user32.dll, ...).
        - Resolves their export tables.
        - Jumps to entry point.
                ↓
   6. Your Rust main() runs.
        - Tauri creates a HWND (window handle).
        - Embeds WebView2 control inside the HWND.
        - WebView2 spawns msedgewebview2.exe helper(s).
        - Loads index.html.
                ↓
   7. App is alive. Visible in the taskbar.
                ↓
   8. User clicks X → app fully exits (different from macOS!).
                ↓
   9. ExitProcess() → kernel reclaims everything.
```

#### Authenticode signing

Same idea as macOS, Microsoft's flavor:

```
   Build:           cargo tauri build → zen-c.exe
       ↓
   Sign:            signtool sign /f cert.pfx /tr <timestamp_url> zen-c.exe
       ↓            (cert ~$300/yr standard, ~$500/yr EV — EV gets instant trust)
   Package:         Tauri produces .msi or .exe installer.
       ↓
   Distribute.
```

Without signing → SmartScreen big red warning → most users won't click through.

### 8.3 The Registry — a Windows concept with no Mac analogue

The **Registry** is Windows's hierarchical key-value config database. The OS and apps store config there.

```
   HKEY_CURRENT_USER\Software\Surfboard\ZenC\
       Version       = "0.1.0"
       LastSession   = "2026-04-30T14:23:00Z"

   HKEY_LOCAL_MACHINE\Software\Microsoft\Windows\CurrentVersion\Uninstall\
       {GUID-of-ZenC} → installer info, used by "Add/Remove Programs"
```

ZenC will mostly avoid the Registry. Tauri stores per-user settings under `%APPDATA%\zen-c\` (a regular folder), not Registry, because folders are easier to back up and don't require admin rights.

### 8.4 Where user data lives

| Data | macOS | Windows |
|---|---|---|
| App data | `~/Library/Application Support/ZenC/` | `%APPDATA%\ZenC\` (= `C:\Users\You\AppData\Roaming\ZenC\`) |
| Cache | `~/Library/Caches/ZenC/` | `%LOCALAPPDATA%\ZenC\Cache\` |
| Logs | `~/Library/Logs/ZenC/` | Same as app data, `\logs\` subdir |
| Config | App data dir | App data dir or Registry |

**Don't hardcode paths.** Use Tauri's `app_data_dir()`, `app_log_dir()`, etc. They return the OS-blessed location automatically.

### 8.5 NestJS world ↔ macOS ↔ Windows comparison

| Concept | NestJS world | macOS | Windows |
|---|---|---|---|
| Deployable artifact | Docker image | `.app` bundle | `.exe` + DLLs (or MSI) |
| "Manifest" | `package.json` | `Info.plist` | PE manifest + Registry |
| Identity proof | Container registry signing | Developer ID + Notarization | Authenticode signature |
| Permission gate | Kubernetes NetworkPolicy | TCC permission prompts | UAC + capabilities |
| Where user data lives | Volume mount | `~/Library/Application Support/` | `%APPDATA%\` |
| Auto-start | systemd unit | LaunchAgent plist | Registry `Run` key / Task Scheduler |
| Logs | `docker logs` | Console.app | Event Viewer |
| Process inspector | `docker stats` | Activity Monitor | Task Manager |
| Kill | `docker kill` | ⌘Q / `kill <pid>` | Task Manager → End Task |

**Verify it yourself:**
- macOS: open Activity Monitor → see all processes with their PIDs.
- macOS: `ls /Applications/Calculator.app/Contents/` to see a real bundle.
- macOS: `cat /Applications/Calculator.app/Contents/Info.plist | head -50` (it's binary plist; you'll see structure even if not fully readable).
- macOS: `codesign -dv --verbose=4 /Applications/Calculator.app` shows the signature info.

---

## Layer 9 — Putting it all together for ZenC

End-to-end trace, combining every layer above.

### 9.1 What `cargo tauri dev` does, step by step

```
  YOU TYPE:  cargo tauri dev
       │
       ▼
  ┌────────────────────────────────────────────────────────────────┐
  │ 1. Vite/npm starts:                                              │
  │      - watches src/*.tsx                                         │
  │      - bundles them to a dev-server URL (localhost:1420)          │
  │      - serves index.html + bundle.js + hot-reload over HTTP      │
  └────────────────────────────────────────────────────────────────┘
       │
       ▼
  ┌────────────────────────────────────────────────────────────────┐
  │ 2. Cargo compiles src-tauri/*.rs:                                │
  │      - rustc → LLVM IR → machine code → Mach-O                    │
  │      - links libSystem.B.dylib, WebKit, etc. dynamically          │
  │      - outputs target/debug/zen-c (a Mach-O binary)               │
  └────────────────────────────────────────────────────────────────┘
       │
       ▼
  ┌────────────────────────────────────────────────────────────────┐
  │ 3. Tauri spawns the binary:                                      │
  │      - macOS launchd creates a process with a fresh PID           │
  │      - dyld maps the Mach-O segments into the new process's RAM   │
  │      - dyld resolves dynamic libraries (libSystem, WebKit)        │
  │      - dyld jumps to the entry point: your fn main()              │
  └────────────────────────────────────────────────────────────────┘
       │
       ▼
  ┌────────────────────────────────────────────────────────────────┐
  │ 4. Your Rust fn main() runs:                                     │
  │      - tauri::Builder::default().run(...)                        │
  │      - asks AppKit for an NSApplication                          │
  │      - asks AppKit for an NSWindow                               │
  │      - inserts a WKWebView into the NSWindow                     │
  │      - WKWebView's host view forwards to OS                      │
  └────────────────────────────────────────────────────────────────┘
       │
       ▼
  ┌────────────────────────────────────────────────────────────────┐
  │ 5. WKWebView spawns helper processes (NEW PIDs):                 │
  │      - com.apple.WebKit.WebContent (sandboxed, runs JS + DOM)    │
  │      - com.apple.WebKit.Networking (sandboxed, fetches HTTP)     │
  │      - com.apple.WebKit.GPU (sandboxed, GPU compositing)          │
  │      - all communicate via XPC (Mach ports) to your zen-c process │
  └────────────────────────────────────────────────────────────────┘
       │
       ▼
  ┌────────────────────────────────────────────────────────────────┐
  │ 6. WebContent loads http://localhost:1420 (Vite dev server):     │
  │      - parses index.html → DOM tree                              │
  │      - fetches bundle.js                                         │
  │      - JavaScriptCore parses bundle.js → bytecode                │
  │      - executes top-level code: ReactDOM.createRoot(...).render  │
  │      - JavaScriptCore JITs hot paths after a few executions      │
  └────────────────────────────────────────────────────────────────┘
       │
       ▼
  ┌────────────────────────────────────────────────────────────────┐
  │ 7. Your React app is mounted. User sees the UI.                  │
  │    Hot reload works because Vite's WebSocket server pushes        │
  │    updated modules over the dev server connection.                │
  └────────────────────────────────────────────────────────────────┘
       │
       ▼
  ┌────────────────────────────────────────────────────────────────┐
  │ 8. User clicks "Start focus session":                            │
  │      - React handler: await invoke('start_focus_session', {...}) │
  │      - JS in WebContent process serializes args to JSON           │
  │      - posts via webkit.messageHandlers (Mach port message)       │
  │      - Tauri (in zen-c process) deserializes                      │
  │      - calls #[tauri::command] fn start_focus_session(...)       │
  │      - that fn writes to SQLite, returns a Session struct         │
  │      - Tauri serializes Session to JSON, sends back               │
  │      - JS Promise resolves; React updates state; UI re-renders    │
  └────────────────────────────────────────────────────────────────┘
       │
       ▼
  ┌────────────────────────────────────────────────────────────────┐
  │ 9. Eventually user closes the window:                            │
  │      - macOS sends close event to NSApplication                  │
  │      - Tauri's run loop returns from main()                      │
  │      - process exits; kernel reclaims memory and helpers          │
  └────────────────────────────────────────────────────────────────┘
```

That's the entire chain — from `cargo tauri dev` to "your React app is talking to your Rust code."

### 9.2 Same trace for production on Windows (abbreviated)

```
  cargo tauri build
       ↓ rustc → PE binary (zen-c.exe)
       ↓ Vite → bundle.js, index.html (embedded in zen-c.exe's resources)
       ↓ tauri-bundler → MSI installer
       ↓ signtool sign zen-c.exe + the MSI
       ↓ ship to users
       ↓
  User installs MSI → C:\Program Files\ZenC\zen-c.exe
       ↓
  User double-clicks → Explorer → CreateProcess →
       SmartScreen → UAC (if needed) → ntdll loader →
       PE mapped into RAM → entry point →
       Rust main() → CreateWindowEx (HWND) →
       WebView2 control loaded → msedgewebview2.exe spawned →
       WebView2 loads embedded index.html →
       V8 parses bundle.js → React mounts → ready
```

Same shape, different vendors at each step. The IPC mechanism switches from Mach ports to Mojo. The window handle switches from NSWindow to HWND. The browser engine switches from WebKit to Chromium. The structure is the same.

### 9.3 The mega-diagram

```
   ┌─────────────────────────────────────────────────────────────────┐
   │                      ZenC running on macOS                         │
   │                                                                    │
   │  ┌──────────────────────────────────────────────────────────────┐ │
   │  │  Process: zen-c (PID 14821)                                   │ │
   │  │  ┌────────────────────────────────────────────────────────┐  │ │
   │  │  │  Native machine code (compiled Rust)                    │  │ │
   │  │  │   - fn main()                                            │  │ │
   │  │  │   - Tauri runtime + command router                       │  │ │
   │  │  │   - tokio runtime + worker threads                       │  │ │
   │  │  │   - sqlx + SQLite connection                             │  │ │
   │  │  │   - your #[tauri::command] functions                     │  │ │
   │  │  │  Threads: main UI thread + async workers + IPC handler   │  │ │
   │  │  │  Heap: holds Vec, Arc, String, your structs              │  │ │
   │  │  └────────────────────────────────────────────────────────┘  │ │
   │  │  ┌────────────────────────────────────────────────────────┐  │ │
   │  │  │  AppKit                                                  │  │ │
   │  │  │   - NSApplication event loop                              │  │ │
   │  │  │   - NSWindow (the window you see)                         │  │ │
   │  │  │   - WKWebView host view (placeholder, forwards to helpers)│  │ │
   │  │  └────────────────────────────────────────────────────────┘  │ │
   │  └──────────────────────────────────────────────────────────────┘ │
   │            │                                                        │
   │            │ XPC (Mach port messages)                               │
   │            ▼                                                        │
   │  ┌──────────────────────────────────────────────────────────────┐ │
   │  │  Process: com.apple.WebKit.WebContent (PID 14822, sandboxed)  │ │
   │  │   - WebKit HTML/CSS engine                                    │ │
   │  │   - JavaScriptCore                                            │ │
   │  │   - Loaded: index.html + bundle.js                            │ │
   │  │   - Running: your React tree                                  │ │
   │  │   - Cannot read your files, your env vars, your DB            │ │
   │  └──────────────────────────────────────────────────────────────┘ │
   │  ┌──────────────────────────────────────────────────────────────┐ │
   │  │  Process: com.apple.WebKit.Networking (PID 14823, sandboxed)  │ │
   │  │   - All HTTP fetches from React go through here.              │ │
   │  └──────────────────────────────────────────────────────────────┘ │
   │  ┌──────────────────────────────────────────────────────────────┐ │
   │  │  Process: com.apple.WebKit.GPU (PID 14824, sandboxed)         │ │
   │  │   - Compositing & GPU command submission.                     │ │
   │  └──────────────────────────────────────────────────────────────┘ │
   │                                                                    │
   │            (all four processes communicate via Mach ports)         │
   │                                                                    │
   │  ───────────────────── kernel boundary ─────────────────────────  │
   │                                                                    │
   │            macOS XNU kernel: scheduler, MMU, syscalls, FS, net     │
   │                                                                    │
   └─────────────────────────────────────────────────────────────────┘
```

This is what's actually running while ZenC is open. One big binary, four processes (one of yours, three WebKit helpers), each talking to the kernel for hardware access, talking to each other via Mach ports.

---

## Layer 10 — Where to go next

You're ready to read `SYSTEM_ARCHITECTURE_AND_LEARNING.md` Part 3 (IPC commands) end-to-end. Every term there now has a concrete meaning.

### Things to actually try on your Mac (all read-only, none change anything)

```bash
# What format is a binary?
file /bin/ls

# What's in its header?
otool -h /bin/ls

# What dylibs does it dynamically link?
otool -L /bin/ls

# What load commands does it have?
otool -l /bin/ls | head -40

# Watch syscalls of a quick command (needs sudo):
sudo dtruss ls 2>&1 | head -20

# See all running processes:
ps -ef | head

# See all WebKit helper processes (open Safari first):
ps -ef | grep WebKit

# See your own login user's app data:
ls ~/Library/Application\ Support/ | head

# See what's signed and how:
codesign -dv --verbose=4 /Applications/Calculator.app
```

Open Activity Monitor and watch the WebKit helpers appear when you open Safari, and disappear when you close it. That's what ZenC will do too.

### What you'll learn next when we start Phase 1

We'll wire the first real `#[tauri::command]` function: `start_focus_session`. Now that you know what's *actually happening* when React calls it — JSON serialization, Mach port hop into your process, Tauri router lookup, SQL insert, JSON serialize back, JS Promise resolves — the code will read like a script you understand line-by-line, not like magic.
