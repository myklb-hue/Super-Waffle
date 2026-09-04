//! Every built-in kind, in the order SPEC §6 lists them.
//!
//! Read this beside §6: each shelf is a section, each row is an entry, and the
//! only additions are the ones the specification asks for elsewhere and §6 has
//! not caught up with. Those are marked `NOTE`.

use crate::*;
use graph_format::PortType::*;
use graph_format::Side::{In, Out};

/// The whole catalogue. `custom` is last: it is the shelf a user's own blocks
/// land on, and its ports come from the code rather than from here (SPEC §10).
pub static KINDS: &[BlockKind] = &[
    // ---------------------------------------------------------- 6.1 Models
    BlockKind {
        id: "llm",
        title: "LLM",
        category: Category::Models,
        icon: "llm",
        summary: "Any local or remote chat model",
        source: false,
        stage: false,
        ports: &[
            optional("trigger", Exec, In),
            port("prompt", Text, In),
            optional("context", Data, In),
            optional("tools", Tools, In),
            optional("memory", Memory, In),
            port("text", Text, Out),
            // Separate from `text` so reasoning can reach a terminal without
            // reaching the display.
            optional("thoughts", Text, Out),
            optional("calls", Data, Out),
        ],
        settings: &[
            setting("provider", "Provider", SettingKind::Text),
            setting("model", "Model", SettingKind::Text),
            setting("endpoint", "Endpoint", SettingKind::Text),
            select("role", "Role", &["assistant", "orchestrator", "classifier"]),
            range("temperature", "Temperature", 0.0, 2.0),
            range("topP", "Top-p", 0.0, 1.0),
            setting("maxTokens", "Max tokens", SettingKind::Number),
            setting("systemPrompt", "System prompt", SettingKind::Multiline),
        ],
    },
    BlockKind {
        id: "object-detection",
        title: "Object detection",
        category: Category::Models,
        icon: "eye",
        summary: "Labels, confidences and boxes",
        source: false,
        stage: true,
        ports: &[port("image", Image, In), port("objects", Data, Out)],
        settings: &[
            setting("model", "Model", SettingKind::Text),
            range("confidence", "Confidence", 0.0, 1.0),
        ],
    },
    BlockKind {
        id: "face-recognition",
        title: "Face recognition",
        category: Category::Models,
        icon: "approve",
        summary: "Embeddings only, never images",
        source: false,
        stage: true,
        ports: &[port("image", Image, In), port("person", Data, Out)],
        settings: &[
            setting("model", "Model", SettingKind::Text),
            hinted(
                switch("enrolment", "Enrolment", false),
                "Off by default. Adding a person stores 512 floats, never a picture.",
            ),
            // No "store images" switch, deliberately. §12.3 does not say images
            // are off by default, it says faces are stored as embeddings and
            // never as images — a property of the program, not a preference. A
            // switch would have made it a preference, and the panel would have
            // read "never an image" directly above a control offering one.
            falls_back_to(range("threshold", "Match threshold", 0.0, 1.0), "0.8"),
        ],
    },
    BlockKind {
        id: "speech-to-text",
        title: "Speech to text",
        category: Category::Models,
        icon: "note",
        summary: "Streaming transcription, with a lag figure",
        source: false,
        stage: false,
        ports: &[port("audio", Audio, In), port("text", Text, Out)],
        settings: &[
            setting("model", "Model", SettingKind::Text),
            setting("language", "Language", SettingKind::Text),
        ],
    },
    BlockKind {
        id: "text-to-speech",
        title: "Text to speech",
        category: Category::Models,
        icon: "note",
        summary: "A voice for what the model says",
        source: false,
        stage: false,
        ports: &[port("text", Text, In), port("audio", Audio, Out)],
        settings: &[
            setting("voice", "Voice", SettingKind::Text),
            range("rate", "Rate", 0.5, 2.0),
        ],
    },
    BlockKind {
        id: "embedding",
        title: "Embedding",
        category: Category::Models,
        icon: "embed",
        summary: "Text as a vector",
        source: false,
        stage: false,
        ports: &[port("text", Text, In), port("data", Data, Out)],
        settings: &[setting("model", "Model", SettingKind::Text)],
    },
    BlockKind {
        id: "classifier",
        title: "Classifier",
        category: Category::Models,
        icon: "shield",
        summary: "A label with a confidence",
        source: false,
        stage: false,
        ports: &[port("text", Text, In), port("data", Data, Out)],
        settings: &[
            setting("model", "Model", SettingKind::Text),
            setting("labels", "Labels", SettingKind::List),
        ],
    },
    BlockKind {
        id: "affect",
        title: "Affect",
        category: Category::Models,
        icon: "face",
        summary: "Valence and arousal read from a stream of text",
        source: false,
        stage: false,
        // Feeds an Avatar's `express` port, so a smile costs no tool call.
        ports: &[port("text", Text, In), port("affect", Data, Out)],
        settings: &[setting("model", "Model", SettingKind::Text)],
    },
    // ---------------------------------------------------- 6.2 Capabilities
    BlockKind {
        id: "toolbox",
        title: "Toolbox",
        category: Category::Capabilities,
        icon: "toolbox",
        summary: "One slot per connected tool, one bundle out",
        source: false,
        stage: false,
        ports: &[
            dynamic("tools", Tools, In),
            optional("pause", Exec, In),
            port("tools", Tools, Out),
        ],
        settings: &[
            select("toolChoice", "Tool choice", &["auto", "required", "none"]),
            hinted(
                switch("warnBefore", "Warn before dangerous calls", true),
                "A prompt with a Continue. It never blocks the graph.",
            ),
        ],
    },
    BlockKind {
        id: "web-search",
        title: "Web Search",
        category: Category::Capabilities,
        icon: "search",
        summary: "Search the web and return results",
        source: false,
        stage: false,
        ports: &[port("text", Text, In), port("data", Data, Out)],
        settings: &[
            setting("provider", "Provider", SettingKind::Text),
            setting("results", "Results", SettingKind::Number),
        ],
    },
    BlockKind {
        id: "file-system",
        title: "File System",
        category: Category::Capabilities,
        icon: "folder",
        summary: "Read, write and move files",
        source: false,
        stage: false,
        // NOTE: `trigger` is not in the §6.2 row. The inbox-triage example
        // (§13.2) fires an archive move from a Branch, which needs one.
        ports: &[
            optional("trigger", Exec, In),
            port("text", Text, In),
            port("file", File, Out),
        ],
        settings: &[
            select("action", "Action", &["read", "write", "move", "delete"]),
            setting("root", "Root", SettingKind::Path),
            setting("to", "Destination", SettingKind::Path),
        ],
    },
    BlockKind {
        id: "mcp-server",
        title: "MCP Server",
        category: Category::Capabilities,
        icon: "plug",
        summary: "An MCP server's tools as one bundle",
        source: false,
        stage: false,
        ports: &[port("tools", Tools, Out)],
        settings: &[
            setting("command", "Command", SettingKind::Text),
            setting("args", "Arguments", SettingKind::List),
        ],
    },
    // -------------------------------------------------------- 6.3 Runtimes
    BlockKind {
        id: "terminal",
        title: "Terminal",
        category: Category::Runtimes,
        icon: "terminal",
        summary: "Runs a command; offered to a model as terminal.run",
        source: false,
        stage: false,
        // NOTE: `text` is not in the §6.3 row. The assistant example (§13.3)
        // wires the orchestrator's thoughts into a terminal to print them.
        ports: &[
            optional("text", Text, In),
            port("tool", Tools, Out),
            port("stdout", Stream, Out),
        ],
        settings: &[
            setting("command", "Command", SettingKind::Text),
            setting("cwd", "Working directory", SettingKind::Path),
            hinted(
                switch("warnBefore", "Warn before running", true),
                "A shell command is a dangerous action. The prompt always has a Continue.",
            ),
        ],
    },
    BlockKind {
        id: "python",
        title: "Python",
        category: Category::Runtimes,
        icon: "python",
        summary: "python.exec, in the workspace environment",
        source: false,
        stage: false,
        ports: &[port("tool", Tools, Out), port("value", Data, Out)],
        settings: &[
            setting("source", "Source", SettingKind::Path),
            setting("venv", "Environment", SettingKind::Path),
        ],
    },
    BlockKind {
        id: "node",
        title: "Node",
        category: Category::Runtimes,
        icon: "bolt",
        summary: "JavaScript and TypeScript, run by Bun",
        source: false,
        stage: false,
        ports: &[port("tool", Tools, Out), port("value", Data, Out)],
        settings: &[setting("source", "Source", SettingKind::Path)],
    },
    BlockKind {
        id: "sql",
        title: "SQL",
        category: Category::Runtimes,
        icon: "db",
        summary: "A query against a database file",
        source: false,
        stage: false,
        ports: &[port("text", Text, In), port("data", Data, Out)],
        settings: &[
            setting("database", "Database", SettingKind::Path),
            setting("query", "Query", SettingKind::Multiline),
        ],
    },
    BlockKind {
        id: "http-request",
        title: "HTTP Request",
        category: Category::Runtimes,
        icon: "http",
        summary: "One request, one response",
        source: false,
        stage: false,
        ports: &[port("text", Text, In), port("data", Data, Out)],
        settings: &[
            select(
                "method",
                "Method",
                &["GET", "POST", "PUT", "PATCH", "DELETE"],
            ),
            setting("url", "URL", SettingKind::Text),
            setting("headers", "Headers", SettingKind::Multiline),
        ],
    },
    // ---------------------------------------------------------- 6.4 Senses
    BlockKind {
        id: "webcam",
        title: "Webcam",
        category: Category::Senses,
        icon: "eye",
        summary: "Frames from a camera; downstream blocks sample",
        source: true,
        stage: true,
        ports: &[port("frames", Image, Out)],
        settings: &[
            setting("device", "Device", SettingKind::Path),
            setting("resolution", "Resolution", SettingKind::Text),
            setting("fps", "Frame rate", SettingKind::Number),
            hinted(
                switch("store", "Record frames", false),
                "Off. Frames never leave the machine and are not written to disk.",
            ),
        ],
    },
    BlockKind {
        id: "microphone",
        title: "Microphone",
        category: Category::Senses,
        icon: "note",
        summary: "Samples, with a level meter and a voice-activity state",
        source: true,
        stage: false,
        ports: &[port("audio", Audio, Out)],
        settings: &[
            setting("device", "Device", SettingKind::Text),
            switch("vad", "Voice activity", true),
            hinted(
                switch("store", "Record audio", false),
                "Off. Audio never leaves the machine and is not written to disk.",
            ),
        ],
    },
    BlockKind {
        id: "keyboard",
        title: "Keyboard",
        category: Category::Senses,
        icon: "form",
        summary: "A line of typed input",
        source: true,
        stage: false,
        ports: &[port("text", Text, Out)],
        settings: &[setting("placeholder", "Placeholder", SettingKind::Text)],
    },
    BlockKind {
        id: "schedule",
        title: "Schedule",
        category: Category::Senses,
        icon: "clock",
        summary: "Every, cron, or once at",
        source: true,
        stage: false,
        ports: &[port("tick", Exec, Out)],
        settings: &[
            setting("every", "Every", SettingKind::Text),
            setting("cron", "Cron", SettingKind::Text),
            setting("jitter", "Jitter", SettingKind::Number),
            select("catchUp", "Catch up", &["skip", "run-once", "run-all"]),
        ],
    },
    BlockKind {
        id: "watch-folder",
        title: "Watch folder",
        category: Category::Senses,
        icon: "folder",
        summary: "A path, a pattern and a debounce",
        source: true,
        stage: false,
        ports: &[port("file", File, Out)],
        settings: &[
            setting("path", "Path", SettingKind::Path),
            setting("pattern", "Pattern", SettingKind::Text),
            setting("events", "Events", SettingKind::List),
            setting("debounce", "Debounce", SettingKind::Number),
        ],
    },
    BlockKind {
        id: "webhook",
        title: "Webhook",
        category: Category::Senses,
        icon: "http",
        summary: "A method, a path and a port",
        source: true,
        stage: false,
        ports: &[port("event", Data, Out)],
        settings: &[
            falls_back_to(select("method", "Method", &["GET", "POST"]), "POST"),
            setting("path", "Path", SettingKind::Text),
            setting("port", "Port", SettingKind::Number),
        ],
    },
    // ---------------------------------------------------------- 6.5 Memory
    BlockKind {
        id: "memory-hub",
        title: "Memory hub",
        category: Category::Memory,
        icon: "merge",
        summary: "One slot per store, one handle out",
        source: false,
        stage: false,
        ports: &[dynamic("memory", Memory, In), port("memory", Memory, Out)],
        settings: &[
            select(
                "recall",
                "Recall order",
                &["recent-first", "relevance", "mixed"],
            ),
            falls_back_to(
                setting("maxRecalled", "Max recalled", SettingKind::Number),
                "12",
            ),
            hinted(
                falls_back_to(range("cutoff", "Relevance cutoff", 0.0, 1.0), "0.6"),
                "Below this, a memory is noise rather than context.",
            ),
            setting("consolidateEvery", "Consolidate every", SettingKind::Text),
            hinted(
                switch("summarise", "Summarise before storing", true),
                "The orchestrator writes one line per episode on its way to long-term memory.",
            ),
            hinted(
                switch("forgetAfter", "Forget after retention", false),
                "Off. Nothing is deleted by age unless you turn this on.",
            ),
            setting("retention", "Retention", SettingKind::Text),
        ],
    },
    BlockKind {
        id: "working-memory",
        title: "Working memory",
        category: Category::Memory,
        icon: "braces",
        summary: "In process, fast, windowed",
        source: false,
        stage: false,
        ports: &[port("memory", Memory, Out)],
        settings: &[
            setting("items", "Items", SettingKind::Number),
            setting("window", "Window", SettingKind::Text),
        ],
    },
    BlockKind {
        id: "long-term-memory",
        title: "Long-term memory",
        category: Category::Memory,
        icon: "db",
        summary: "SQLite and vectors: people, places, episodes",
        source: false,
        stage: false,
        ports: &[port("memory", Memory, Out)],
        settings: &[
            setting("path", "Database", SettingKind::Path),
            switch("vectors", "Vectors", true),
            hinted(
                setting("retention", "Retention", SettingKind::Text),
                "What is kept, and for how long. Deleting a person deletes every sighting.",
            ),
        ],
    },
    BlockKind {
        id: "episode-log",
        title: "Episode log",
        category: Category::Memory,
        icon: "chunk",
        summary: "An append-only record of what happened",
        source: false,
        stage: false,
        ports: &[port("data", Data, In), port("memory", Memory, Out)],
        settings: &[setting("path", "File", SettingKind::Path)],
    },
    // ------------------------------------------------------- 6.6 Actuators
    BlockKind {
        id: "display",
        title: "Display",
        category: Category::Actuators,
        icon: "note",
        summary: "A screen or an overlay",
        source: false,
        stage: true,
        ports: &[port("text", Text, In)],
        settings: &[select("target", "Target", &["window", "overlay"])],
    },
    BlockKind {
        id: "speaker",
        title: "Speaker",
        category: Category::Actuators,
        icon: "note",
        summary: "Plays what it is given",
        source: false,
        stage: false,
        ports: &[port("audio", Audio, In)],
        settings: &[
            setting("device", "Device", SettingKind::Text),
            range("volume", "Volume", 0.0, 1.0),
        ],
    },
    BlockKind {
        id: "usb-device",
        title: "USB device",
        category: Category::Actuators,
        icon: "plug",
        summary: "A serial device: usb.send, usb.read",
        source: false,
        stage: false,
        ports: &[port("tool", Tools, Out), port("read", Stream, Out)],
        settings: &[
            setting("port", "Port", SettingKind::Path),
            setting("baud", "Baud", SettingKind::Number),
        ],
    },
    BlockKind {
        id: "motors",
        title: "Motors",
        category: Category::Actuators,
        icon: "bolt",
        summary: "A servo controller: motor.move, motor.home",
        source: false,
        stage: false,
        // The three shapes of feedback in one block: the tool replies on its
        // own handle, `state` streams telemetry, `fault` interrupts (SPEC §4.3).
        ports: &[
            port("tool", Tools, Out),
            port("state", Stream, Out),
            port("fault", Exec, Out),
        ],
        settings: &[
            setting("port", "Port", SettingKind::Path),
            setting("panLimit", "Pan limit", SettingKind::Number),
            setting("tiltLimit", "Tilt limit", SettingKind::Number),
            hinted(
                switch("warnBeforeMove", "Warn before moving", true),
                "A move is a dangerous action. The prompt always has a Continue.",
            ),
        ],
    },
    BlockKind {
        id: "gpio",
        title: "GPIO",
        category: Category::Actuators,
        icon: "plug",
        summary: "Pins in and out",
        source: false,
        stage: false,
        ports: &[port("tool", Tools, Out), port("pins", Stream, Out)],
        settings: &[setting("chip", "Chip", SettingKind::Path)],
    },
    BlockKind {
        id: "avatar",
        title: "Avatar",
        category: Category::Actuators,
        icon: "face",
        summary: "The assistant's presence: a rig, a vocabulary and a face",
        source: false,
        stage: true,
        ports: &[
            port("speech", Audio, In),
            port("express", Data, In),
            optional("look", Data, In),
            optional("tool", Tools, Out),
            optional("state", Stream, Out),
        ],
        settings: &[
            select("rig", "Rig", &["line", "robot", "orb", "pixel"]),
            hinted(
                switch("autoAffectFromSpeech", "Auto-affect from speech", true),
                "Off: an Affect block feeds express instead.",
            ),
            setting("blink", "Blink", SettingKind::Text),
            setting("breathePerMin", "Breathe", SettingKind::Number),
            setting("settleSec", "Settle to neutral after", SettingKind::Number),
            setting("sleepAfterMin", "Sleep after", SettingKind::Number),
            switch("keepAspect", "Keep aspect", true),
        ],
    },
    BlockKind {
        id: "status-light",
        title: "Status light",
        category: Category::Actuators,
        icon: "lamp",
        summary: "A lamp that breathes a colour on the same vocabulary",
        source: false,
        stage: false,
        ports: &[port("express", Data, In), optional("tool", Tools, Out)],
        settings: &[setting("device", "Device", SettingKind::Text)],
    },
    BlockKind {
        id: "sound-cue",
        title: "Sound cue",
        category: Category::Actuators,
        icon: "bell",
        summary: "A chime per expression",
        source: false,
        stage: false,
        ports: &[port("express", Data, In), optional("tool", Tools, Out)],
        settings: &[setting("pack", "Sound pack", SettingKind::Path)],
    },
    // ------------------------------------------------------------ 6.7 Data
    BlockKind {
        id: "input",
        title: "Input",
        category: Category::Data,
        icon: "input",
        summary: "The graph's entry value",
        source: false,
        stage: false,
        // NOTE: §6.7 gives the type (`any`) but no port name; `value` is the
        // name used here and in the fixtures. The Main artboard labels this
        // port `text`, which predates the catalogue.
        ports: &[port("value", Any, Out)],
        settings: &[setting("value", "Value", SettingKind::Multiline)],
    },
    BlockKind {
        id: "output",
        title: "Output",
        category: Category::Data,
        icon: "output",
        summary: "A named result",
        source: false,
        stage: false,
        ports: &[port("value", Any, In)],
        settings: &[setting("name", "Name", SettingKind::Text)],
    },
    BlockKind {
        id: "variable",
        title: "Variable",
        category: Category::Data,
        icon: "braces",
        summary: "Holds a value between events",
        source: false,
        stage: false,
        ports: &[port("value", Any, In), port("value", Any, Out)],
        settings: &[setting("name", "Name", SettingKind::Text)],
    },
    BlockKind {
        id: "chunker",
        title: "Chunker",
        category: Category::Data,
        icon: "chunk",
        summary: "Splits text into pieces",
        source: false,
        stage: false,
        ports: &[port("text", Text, In), port("text", Text, Out)],
        settings: &[
            setting("size", "Size", SettingKind::Number),
            setting("overlap", "Overlap", SettingKind::Number),
        ],
    },
    BlockKind {
        id: "secret",
        title: "Secret",
        category: Category::Data,
        icon: "key",
        summary: "Bound from the graph's env by name, never by value",
        source: false,
        stage: false,
        ports: &[port("text", Text, Out)],
        settings: &[hinted(
            setting("name", "Name", SettingKind::Text),
            "The name of a secret in the OS keyring. The value is never written to the file.",
        )],
    },
    BlockKind {
        id: "convert",
        title: "Convert",
        category: Category::Data,
        icon: "merge",
        summary: "Makes one type into another, visibly",
        source: false,
        stage: false,
        // NOTE: added by SPEC §15.5, which replaced the transform-on-a-wire
        // idea with a block you can see. Not yet on the Library artboard.
        ports: &[port("value", Any, In), port("value", Any, Out)],
        settings: &[select(
            "to",
            "To",
            &["text", "data", "file", "image", "audio"],
        )],
    },
    // --------------------------------------------------------- 6.8 Control
    BlockKind {
        id: "loop",
        title: "Loop",
        category: Category::Control,
        icon: "loop",
        summary: "A frame, not a card: blocks inside repeat per item",
        source: false,
        stage: false,
        // NOTE: `item` is not in the §6.8 row, but §13.2's wire table uses it
        // ("Loop item → Classify prompt") and a loop is useless without it:
        // it is how the blocks inside the frame receive the current item.
        ports: &[
            port("items", Any, In),
            port("item", Any, Out),
            port("results", Data, Out),
            optional("done", Exec, Out),
            optional("errors", Data, Out),
        ],
        settings: &[
            setting("as", "Item name", SettingKind::Text),
            falls_back_to(setting("parallel", "Parallel", SettingKind::Number), "2"),
            setting("max", "Max items", SettingKind::Number),
            switch("continueOnError", "Continue on error", true),
        ],
    },
    BlockKind {
        id: "branch",
        title: "Branch",
        category: Category::Control,
        icon: "branch",
        summary: "A condition on the input",
        source: false,
        stage: false,
        ports: &[
            port("value", Any, In),
            port("a", Exec, Out),
            port("b", Exec, Out),
        ],
        settings: &[setting("condition", "Condition", SettingKind::Text)],
    },
    BlockKind {
        id: "merge",
        title: "Merge",
        category: Category::Control,
        icon: "merge",
        summary: "Several inputs, one output",
        source: false,
        stage: false,
        ports: &[dynamic("value", Any, In), port("value", Any, Out)],
        settings: &[select("mode", "Mode", &["first", "all", "concat"])],
    },
    BlockKind {
        id: "gate",
        title: "Gate",
        category: Category::Control,
        icon: "shield",
        summary: "Holds a value until it is opened",
        source: false,
        stage: false,
        ports: &[
            port("value", Any, In),
            port("open", Exec, In),
            port("value", Any, Out),
        ],
        settings: &[],
    },
    BlockKind {
        id: "delay",
        title: "Delay",
        category: Category::Control,
        icon: "clock",
        summary: "Passes a value on, later",
        source: false,
        stage: false,
        ports: &[port("value", Any, In), port("value", Any, Out)],
        settings: &[setting("ms", "Delay", SettingKind::Number)],
    },
    // ----------------------------------------------------------- 6.9 Human
    BlockKind {
        id: "approval",
        title: "Approval",
        category: Category::Human,
        icon: "approve",
        summary: "A human step the user chose to place",
        source: false,
        stage: false,
        ports: &[
            port("value", Any, In),
            port("value", Any, Out),
            port("halt", Exec, Out),
        ],
        settings: &[hinted(
            setting("prompt", "Prompt", SettingKind::Text),
            "This block is a choice, not a gate the application imposes (SPEC 12).",
        )],
    },
    BlockKind {
        id: "form",
        title: "Form",
        category: Category::Human,
        icon: "form",
        summary: "Asks for fields and returns a record",
        source: false,
        stage: false,
        ports: &[port("data", Data, Out)],
        settings: &[setting("fields", "Fields", SettingKind::List)],
    },
    BlockKind {
        id: "notify",
        title: "Notify",
        category: Category::Human,
        icon: "bell",
        summary: "Slack, email or desktop",
        source: false,
        stage: false,
        ports: &[optional("send", Exec, In), port("text", Text, In)],
        settings: &[
            select("target", "Target", &["desktop", "slack", "email"]),
            setting("channel", "Channel", SettingKind::Text),
            setting("to", "To", SettingKind::Text),
        ],
    },
    // ---------------------------------------------------------- 6.10 Custom
    BlockKind {
        id: "custom",
        title: "Custom",
        category: Category::Custom,
        icon: "code",
        summary: "Your own code; its signature is its interface",
        source: false,
        stage: false,
        // A custom block's real ports are parsed from its source and stored on
        // the block, not here (SPEC §10.2).
        ports: &[],
        settings: &[],
    },
];

/// Look a kind up by the key a `.loom` file writes.
pub fn kind(id: &str) -> Option<&'static BlockKind> {
    KINDS.iter().find(|k| k.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn ids_are_unique() {
        let mut seen = HashSet::new();
        for k in KINDS {
            assert!(seen.insert(k.id), "duplicate kind id `{}`", k.id);
        }
    }

    /// A port name has to be unique per side. `variable` and `chunker` reuse
    /// one name across the two sides on purpose, which is legal.
    #[test]
    fn port_names_are_unique_per_side() {
        for k in KINDS {
            for side in [Side::In, Side::Out] {
                let mut seen = HashSet::new();
                for p in k.ports.iter().filter(|p| p.side == side) {
                    assert!(
                        seen.insert(p.name),
                        "{}: duplicate {:?} port `{}`",
                        k.id,
                        side,
                        p.name
                    );
                }
            }
        }
    }

    #[test]
    fn setting_names_are_unique() {
        for k in KINDS {
            let mut seen = HashSet::new();
            for s in k.settings {
                assert!(
                    seen.insert(s.name),
                    "{}: duplicate setting `{}`",
                    k.id,
                    s.name
                );
            }
        }
    }

    /// Every icon a kind names has to exist in the shared set, or the library
    /// draws a fallback and nobody notices until it ships.
    #[test]
    fn icons_come_from_the_shared_set() {
        let icons = include_str!("../../../design/cyberloom/icons.mjs");
        for k in KINDS {
            assert!(
                icons.contains(&format!("\n  {}:", k.id))
                    || icons.contains(&format!("\n  {}:", k.icon)),
                "{}: icon `{}` is not in design/cyberloom/icons.mjs",
                k.id,
                k.icon
            );
        }
    }

    /// A source keeps the graph armed, so the set of them is worth pinning:
    /// this is what makes a graph never finish (SPEC §8.2).
    #[test]
    fn the_sources_are_the_senses() {
        let sources: Vec<_> = KINDS.iter().filter(|k| k.source).map(|k| k.id).collect();
        assert_eq!(
            sources,
            [
                "webcam",
                "microphone",
                "keyboard",
                "schedule",
                "watch-folder",
                "webhook"
            ]
        );
        for k in KINDS.iter().filter(|k| k.source) {
            assert_eq!(
                k.category,
                Category::Senses,
                "{} is a source but not a sense",
                k.id
            );
        }
    }

    /// Only a block with a picture offers a Stage view (SPEC §3.4).
    #[test]
    fn stage_is_only_for_blocks_with_a_picture() {
        let staged: Vec<_> = KINDS.iter().filter(|k| k.stage).map(|k| k.id).collect();
        assert_eq!(
            staged,
            [
                "object-detection",
                "face-recognition",
                "webcam",
                "display",
                "avatar"
            ]
        );
    }

    /// A dynamic port is how a Toolbox and a Memory hub grow slots; nothing
    /// else in the catalogue should have one by accident.
    #[test]
    fn only_bundling_blocks_have_dynamic_ports() {
        let dynamic: Vec<_> = KINDS
            .iter()
            .filter(|k| k.ports.iter().any(|p| p.dynamic))
            .map(|k| k.id)
            .collect();
        assert_eq!(dynamic, ["toolbox", "memory-hub", "merge"]);
    }

    /// The catalogue covers every category, so no shelf is empty.
    #[test]
    fn every_category_has_at_least_one_kind() {
        for c in Category::ALL {
            assert!(
                KINDS.iter().any(|k| k.category == c),
                "no kinds in {}",
                c.as_str()
            );
        }
    }

    /// A range setting without bounds would draw a slider with no ends.
    /// A switch has no unset position, so leaving one without a default would
    /// make the inspector pick a side on its own — which is the whole defect
    /// this field exists to fix.
    #[test]
    fn every_switch_says_which_way_it_starts() {
        for k in KINDS {
            for s in k.settings {
                if s.kind == SettingKind::Bool {
                    assert!(
                        matches!(s.default, Some("true" | "false")),
                        "{}.{} is a switch with no side: {:?}",
                        k.id,
                        s.name,
                        s.default
                    );
                }
            }
        }
    }

    /// A segmented control always shows something selected, so its default has
    /// to be one of the things it can show.
    #[test]
    fn every_choice_defaults_to_one_of_its_own_options() {
        for k in KINDS {
            for s in k.settings {
                if s.kind == SettingKind::Select {
                    let default = s.default.unwrap_or_else(|| {
                        panic!("{}.{} is a choice with no default", k.id, s.name)
                    });
                    assert!(
                        s.options.contains(&default),
                        "{}.{} falls back to `{default}`, which is not one of {:?}",
                        k.id,
                        s.name,
                        s.options
                    );
                }
            }
        }
    }

    /// A default outside a slider's own range would put the handle off the end
    /// of the track.
    #[test]
    fn a_slider_default_is_inside_its_bounds() {
        for k in KINDS {
            for s in k.settings {
                let (Some(default), Some(min), Some(max)) = (s.default, s.min, s.max) else {
                    continue;
                };
                let value: f64 = default.parse().unwrap_or_else(|_| {
                    panic!(
                        "{}.{} falls back to `{default}`, which is not a number",
                        k.id, s.name
                    )
                });
                assert!(
                    (min..=max).contains(&value),
                    "{}.{} falls back to {value}, outside {min}..{max}",
                    k.id,
                    s.name
                );
            }
        }
    }

    /// A number's default has to be a number, or the field shows text where a
    /// figure belongs.
    #[test]
    fn a_number_default_is_a_number() {
        for k in KINDS {
            for s in k.settings {
                if s.kind == SettingKind::Number
                    && let Some(default) = s.default
                {
                    assert!(
                        default.parse::<f64>().is_ok(),
                        "{}.{} falls back to `{default}`",
                        k.id,
                        s.name
                    );
                }
            }
        }
    }

    /// Nothing invents a default for a control that can be left alone. A
    /// temperature has none on purpose: the engine leaves it out of the
    /// request and the provider's own default applies, which is a better
    /// answer than a number chosen here.
    #[test]
    fn a_setting_that_can_be_unset_is_left_unset() {
        let llm = kind("llm").unwrap();
        assert_eq!(llm.setting("temperature").unwrap().default, None);
        assert_eq!(llm.setting("topP").unwrap().default, None);
        assert_eq!(llm.setting("systemPrompt").unwrap().default, None);
        // But its role is a choice, so it has one.
        assert_eq!(llm.setting("role").unwrap().default, Some("assistant"));
    }

    /// The defaults SPEC states, checked against the specification rather than
    /// against whatever the table happens to say.
    #[test]
    fn the_defaults_the_specification_states() {
        // §6.1: "enrolment is off by default".
        let face = kind("face-recognition").unwrap();
        assert_eq!(face.setting("enrolment").unwrap().default, Some("false"));
        // §12.3: faces are stored as embeddings, never images. Unconditionally
        // — so what the catalogue owes is the absence of any way to ask for an
        // image, not a switch that starts off.
        assert!(
            face.settings
                .iter()
                .all(|def| !def.name.to_lowercase().contains("image")),
            "face recognition has a setting about images: {:?}",
            face.settings.iter().map(|d| d.name).collect::<Vec<_>>()
        );
        // §12.3: frames and audio are not recorded unless the user turns it on.
        assert_eq!(
            kind("webcam").unwrap().setting("store").unwrap().default,
            Some("false")
        );
        assert_eq!(
            kind("microphone")
                .unwrap()
                .setting("store")
                .unwrap()
                .default,
            Some("false")
        );
        // §12.2: a shell command and a physical action both warrant a warning.
        assert_eq!(
            kind("terminal")
                .unwrap()
                .setting("warnBefore")
                .unwrap()
                .default,
            Some("true")
        );
        assert_eq!(
            kind("motors")
                .unwrap()
                .setting("warnBeforeMove")
                .unwrap()
                .default,
            Some("true")
        );
        // §8.3: two items in parallel per loop frame.
        assert_eq!(
            kind("loop").unwrap().setting("parallel").unwrap().default,
            Some("2")
        );
    }

    #[test]
    fn range_settings_have_bounds() {
        for k in KINDS {
            for s in k.settings {
                if s.kind == SettingKind::Range {
                    assert!(
                        s.min.is_some() && s.max.is_some(),
                        "{}.{} is a range with no bounds",
                        k.id,
                        s.name
                    );
                }
            }
        }
    }
}
