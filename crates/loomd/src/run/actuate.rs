//! Blocks that act on the world, and what they report back (SPEC §6.6, §4.4).
//!
//! A device that can be commanded can also report, in three shapes: the tool
//! call replies on its own handle, telemetry streams from the device, and a
//! fault interrupts. Motors carry all three, which is why the assistant example
//! uses them to show the whole vocabulary at once.
//!
//! The interrupt is the reflex path. A `fault` wired into a Toolbox's `pause`
//! stops tool calls before the orchestrator has finished its next thought —
//! sooner than a model could react, because it does not go through the model at
//! all. It pauses; it never locks (§9.1).
//!
//! As with every other device in the engine there are two implementations: a
//! real serial port, and a scripted one that answers from a list. A machine
//! with no servo controller can still run the graph, and a test can pin down
//! what a stalled motor does without owning a motor that stalls.

use std::io::{BufRead, BufReader, Write};
use std::sync::Mutex;
use std::time::Duration;

/// What came back from a device.
#[derive(Debug, Clone, PartialEq)]
pub struct Reply {
    /// What to tell the model.
    pub text: String,
    /// Telemetry for the `state` port, when the device said something about
    /// where it is (SPEC §4.4).
    pub state: Option<String>,
    /// Set when the device raised a fault. The `fault` port fires and, if it
    /// is wired to one, a Toolbox stops taking calls.
    pub fault: Option<String>,
}

impl Reply {
    pub fn said(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            state: None,
            fault: None,
        }
    }

    pub fn with_state(mut self, state: impl Into<String>) -> Self {
        self.state = Some(state.into());
        self
    }

    pub fn faulted(mut self, why: impl Into<String>) -> Self {
        self.fault = Some(why.into());
        self
    }
}

/// A device on the end of a wire (SPEC §6.6).
pub trait Device: Send + Sync {
    /// Send one line and read one line back.
    fn send(&self, line: &str) -> Result<String, String>;

    /// What the panel calls it.
    fn describe(&self) -> String;
}

// ------------------------------------------------------------------- serial

/// A real serial port.
///
/// One line out, one line in, which is what every hobby servo controller and
/// microcontroller sketch speaks. A device that says nothing within the timeout
/// has not necessarily failed — it may simply have nothing to say — so a silent
/// reply is an empty string rather than an error, and the caller decides
/// whether silence was expected.
pub struct Serial {
    port: Mutex<Box<dyn serialport::SerialPort>>,
    name: String,
    baud: u32,
}

impl Serial {
    pub fn open(path: &str, baud: u32, timeout: Duration) -> Result<Self, String> {
        let port = serialport::new(path, baud)
            .timeout(timeout)
            .open()
            .map_err(|e| format!("could not open {path}: {e}"))?;
        Ok(Self {
            port: Mutex::new(port),
            name: path.to_owned(),
            baud,
        })
    }
}

impl Device for Serial {
    fn send(&self, line: &str) -> Result<String, String> {
        let mut port = self.port.lock().unwrap();
        writeln!(port, "{line}").map_err(|e| format!("could not write to {}: {e}", self.name))?;
        port.flush()
            .map_err(|e| format!("could not write to {}: {e}", self.name))?;
        let mut reply = String::new();
        // The port is cloned rather than borrowed because `BufReader` wants to
        // own its source and the lock has to be given back afterwards.
        let readable = port
            .try_clone()
            .map_err(|e| format!("could not read from {}: {e}", self.name))?;
        match BufReader::new(readable).read_line(&mut reply) {
            Ok(_) => Ok(reply.trim().to_owned()),
            // A timeout is silence, not failure.
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => Ok(String::new()),
            Err(e) => Err(format!("could not read from {}: {e}", self.name)),
        }
    }

    fn describe(&self) -> String {
        format!("{} · {} baud", self.name, self.baud)
    }
}

// ----------------------------------------------------------------- scripted

/// A device that answers from a list, and remembers what it was told.
///
/// Not a mock of a serial port: it is the same interface with a different thing
/// behind it, which is what lets a graph with motors in it run on a laptop.
#[derive(Default)]
pub struct Scripted {
    pub replies: Mutex<std::collections::VecDeque<String>>,
    pub sent: Mutex<Vec<String>>,
}

impl Scripted {
    pub fn new(replies: impl IntoIterator<Item = &'static str>) -> Self {
        Self {
            replies: Mutex::new(replies.into_iter().map(str::to_owned).collect()),
            sent: Mutex::new(Vec::new()),
        }
    }
}

impl Device for Scripted {
    fn send(&self, line: &str) -> Result<String, String> {
        self.sent.lock().unwrap().push(line.to_owned());
        Ok(self
            .replies
            .lock()
            .unwrap()
            .pop_front()
            // A controller with nothing more to say says ok, the way a real one
            // acknowledges a command it had no comment on.
            .unwrap_or_else(|| "ok".to_owned()))
    }

    fn describe(&self) -> String {
        "scripted".into()
    }
}

// ------------------------------------------------------------------- motors

/// Where a servo controller is pointing.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Aim {
    pub pan: f64,
    pub tilt: f64,
}

impl Aim {
    /// The `state` port's line: telemetry a person and a model can both read.
    pub fn line(&self) -> String {
        format!("pan {:.0}° · tilt {:.0}°", self.pan, self.tilt)
    }
}

/// What the limits allow, and what they do not (SPEC §6.6).
///
/// A limit is not the application overruling the user — §12.1 is clear that it
/// may not — it is the machine's own geometry. Asking a servo for an angle it
/// does not have is how a servo is broken, so the move does not happen and the
/// block raises a fault, which is exactly what a real controller does when it
/// hits an end stop. The user owns the limits: they are two settings, and
/// widening them is a text field away.
pub fn within(aim: Aim, pan_limit: Option<f64>, tilt_limit: Option<f64>) -> Result<(), String> {
    if let Some(limit) = pan_limit
        && aim.pan.abs() > limit
    {
        return Err(format!(
            "pan {:.0}° is past the {:.0}° limit",
            aim.pan, limit
        ));
    }
    if let Some(limit) = tilt_limit
        && aim.tilt.abs() > limit
    {
        return Err(format!(
            "tilt {:.0}° is past the {:.0}° limit",
            aim.tilt, limit
        ));
    }
    Ok(())
}

/// Read what a controller said back.
///
/// The convention is one word then the rest: `ok`, `at <pan> <tilt>`, or
/// `fault <why>`. Anything else is passed through as something the device said,
/// because a controller nobody wrote is allowed to be chatty and a graph should
/// not fall over because it was.
pub fn interpret(said: &str, aimed: Aim) -> Reply {
    let said = said.trim();
    let (word, rest) = said.split_once(' ').unwrap_or((said, ""));
    match word {
        "fault" | "FAULT" => Reply::said(format!("the motors reported a fault: {rest}"))
            .faulted(if rest.is_empty() { "fault" } else { rest }),
        "at" => {
            let mut numbers = rest
                .split_whitespace()
                .filter_map(|n| n.parse::<f64>().ok());
            let at = Aim {
                pan: numbers.next().unwrap_or(aimed.pan),
                tilt: numbers.next().unwrap_or(aimed.tilt),
            };
            Reply::said(format!("moved: {}", at.line())).with_state(at.line())
        }
        // Silence from a controller that took the command is a controller that
        // took the command.
        "" | "ok" | "OK" => {
            Reply::said(format!("moved: {}", aimed.line())).with_state(aimed.line())
        }
        _ => Reply::said(said.to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_move_within_the_limits_is_allowed() {
        assert!(
            within(
                Aim {
                    pan: -40.0,
                    tilt: 0.0
                },
                Some(90.0),
                Some(30.0)
            )
            .is_ok()
        );
    }

    /// SPEC §13.3's move is `motor.move(pan: −40)`, which a 30° pan limit
    /// refuses — and says why in the words a person would use.
    #[test]
    fn a_move_past_a_limit_says_which_limit_and_by_how_much() {
        let refused = within(
            Aim {
                pan: -40.0,
                tilt: 0.0,
            },
            Some(30.0),
            None,
        )
        .unwrap_err();
        assert!(refused.contains("pan"), "{refused}");
        assert!(refused.contains("40"), "{refused}");
        assert!(refused.contains("30"), "{refused}");
    }

    #[test]
    fn no_limit_set_means_no_limit() {
        assert!(
            within(
                Aim {
                    pan: 900.0,
                    tilt: 900.0
                },
                None,
                None
            )
            .is_ok()
        );
    }

    #[test]
    fn a_controller_that_reports_a_fault_is_understood_as_one() {
        let reply = interpret("fault stalled at -31", Aim::default());
        assert_eq!(reply.fault.as_deref(), Some("stalled at -31"));
        assert!(reply.text.contains("stalled at -31"));
    }

    /// §4.3's example, exactly: `motor.move()` returns "stalled at −31°".
    #[test]
    fn a_position_report_becomes_telemetry() {
        let reply = interpret("at -40 12", Aim::default());
        assert_eq!(reply.state.as_deref(), Some("pan -40° · tilt 12°"));
        assert!(reply.fault.is_none());
    }

    #[test]
    fn silence_means_it_went_where_it_was_asked_to_go() {
        let aimed = Aim {
            pan: -40.0,
            tilt: 0.0,
        };
        for said in ["", "ok", "OK"] {
            let reply = interpret(said, aimed);
            assert_eq!(
                reply.state.as_deref(),
                Some("pan -40° · tilt 0°"),
                "for {said:?}"
            );
        }
    }

    #[test]
    fn anything_else_the_device_says_is_passed_through() {
        let reply = interpret("calibrating, hold on", Aim::default());
        assert_eq!(reply.text, "calibrating, hold on");
        assert!(reply.state.is_none() && reply.fault.is_none());
    }

    #[test]
    fn a_scripted_device_remembers_what_it_was_told() {
        let device = Scripted::new(["at -40 0"]);
        assert_eq!(device.send("move -40 0").unwrap(), "at -40 0");
        assert_eq!(device.send("home").unwrap(), "ok");
        assert_eq!(*device.sent.lock().unwrap(), ["move -40 0", "home"]);
    }
}
