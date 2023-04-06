use sdl2::keyboard::Keycode;
use serde::de::Visitor;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::Formatter;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputConfig {
    pub up: String,
    pub down: String,
    pub left: String,
    pub right: String,
    pub a: String,
    pub b: String,
    pub start: String,
    pub select: String,
}

impl Default for InputConfig {
    fn default() -> Self {
        Self {
            up: Keycode::Up.name(),
            down: Keycode::Down.name(),
            left: Keycode::Left.name(),
            right: Keycode::Right.name(),
            a: Keycode::Z.name(),
            b: Keycode::X.name(),
            start: Keycode::Return.name(),
            select: Keycode::RShift.name(),
        }
    }
}

impl std::fmt::Display for InputConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Up={}, Down={}, Left={}, Right={}, A={}, B={}, Start={}, Select={}",
            self.up, self.down, self.left, self.right, self.a, self.b, self.start, self.select
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HotkeyConfig {
    pub exit: Option<String>,
    pub toggle_fullscreen: Option<String>,
    pub save_state: Option<String>,
    pub load_state: Option<String>,
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self {
            exit: Some(Keycode::Escape.name()),
            toggle_fullscreen: Some(Keycode::F9.name()),
            save_state: Some(Keycode::F5.name()),
            load_state: Some(Keycode::F6.name()),
        }
    }
}

impl std::fmt::Display for HotkeyConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Exit={}, ToggleFullscreen={}, SaveState={}, LoadState={}",
            fmt_option(self.exit.as_ref()),
            fmt_option(self.toggle_fullscreen.as_ref()),
            fmt_option(self.save_state.as_ref()),
            fmt_option(self.load_state.as_ref())
        )
    }
}

fn fmt_option<T: std::fmt::Display>(option: Option<&T>) -> String {
    match option {
        Some(value) => format!("{value}"),
        None => "<None>".into(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ControllerInput {
    Button(u8),
    AxisNegative(u8),
    AxisPositive(u8),
}

impl std::fmt::Display for ControllerInput {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Button(button) => write!(f, "Button {button}"),
            Self::AxisNegative(axis) => write!(f, "Axis {axis} -"),
            Self::AxisPositive(axis) => write!(f, "Axis {axis} +"),
        }
    }
}

impl FromStr for ControllerInput {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.to_ascii_lowercase();
        let split: Vec<_> = s.split(' ').collect();
        match split.as_slice() {
            ["button", idx] => {
                let idx: u8 = idx
                    .parse()
                    .map_err(|_err| format!("invalid button index: '{idx}'"))?;
                Ok(Self::Button(idx))
            }
            ["axis", idx, "+"] => {
                let idx: u8 = idx
                    .parse()
                    .map_err(|_err| format!("invalid axis index: '{idx}'"))?;
                Ok(Self::AxisPositive(idx))
            }
            ["axis", idx, "-"] => {
                let idx: u8 = idx
                    .parse()
                    .map_err(|_err| format!("invalid axis index: '{idx}'"))?;
                Ok(Self::AxisNegative(idx))
            }
            _ => Err(format!("invalid controller input string: '{s}'")),
        }
    }
}

impl Serialize for ControllerInput {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

struct ControllerInputVisitor;

impl<'de> Visitor<'de> for ControllerInputVisitor {
    type Value = ControllerInput;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        write!(formatter, "a string representing a ControllerInput")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let input = v.parse().map_err(de::Error::custom)?;
        Ok(input)
    }
}

impl<'de> Deserialize<'de> for ControllerInput {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(ControllerInputVisitor)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControllerConfig {
    pub up: Option<ControllerInput>,
    pub down: Option<ControllerInput>,
    pub left: Option<ControllerInput>,
    pub right: Option<ControllerInput>,
    pub a: Option<ControllerInput>,
    pub b: Option<ControllerInput>,
    pub start: Option<ControllerInput>,
    pub select: Option<ControllerInput>,
    pub axis_deadzone: u16,
}

impl Default for ControllerConfig {
    fn default() -> Self {
        Self {
            up: None,
            down: None,
            left: None,
            right: None,
            a: None,
            b: None,
            start: None,
            select: None,
            axis_deadzone: 5000,
        }
    }
}

impl std::fmt::Display for ControllerConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Up={}, Down={}, Left={}, Right={}, A={}, B={}, Start={}, Select={}, Deadzone={}",
            fmt_option(self.up.as_ref()),
            fmt_option(self.down.as_ref()),
            fmt_option(self.left.as_ref()),
            fmt_option(self.right.as_ref()),
            fmt_option(self.a.as_ref()),
            fmt_option(self.b.as_ref()),
            fmt_option(self.start.as_ref()),
            fmt_option(self.select.as_ref()),
            self.axis_deadzone
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColorScheme {
    BlackAndWhite,
    GreenTint,
}

impl Default for ColorScheme {
    fn default() -> Self {
        Self::BlackAndWhite
    }
}

impl std::fmt::Display for ColorScheme {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BlackAndWhite => write!(f, "BlackAndWhite"),
            Self::GreenTint => write!(f, "GreenTint"),
        }
    }
}

impl FromStr for ColorScheme {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "BlackAndWhite" => Ok(Self::BlackAndWhite),
            "GreenTint" => Ok(Self::GreenTint),
            _ => Err(format!("invalid ColorScheme string: '{s}'")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RunConfig {
    pub gb_file_path: String,
    pub audio_enabled: bool,
    pub sync_to_audio: bool,
    pub vsync_enabled: bool,
    pub launch_fullscreen: bool,
    pub borderless_fullscreen: bool,
    pub force_integer_scaling: bool,
    pub window_width: u32,
    pub window_height: u32,
    pub audio_debugging_enabled: bool,
    pub audio_60hz: bool,
    pub color_scheme: ColorScheme,
    pub input_config: InputConfig,
    pub hotkey_config: HotkeyConfig,
    pub controller_config: ControllerConfig,
}

impl std::fmt::Display for RunConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "gb_file_path: {}", self.gb_file_path)?;
        writeln!(f, "audio_enabled: {}", self.audio_enabled)?;
        writeln!(f, "sync_to_audio: {}", self.sync_to_audio)?;
        writeln!(f, "vsync_enabled: {}", self.vsync_enabled)?;
        writeln!(f, "launch_fullscreen: {}", self.launch_fullscreen)?;
        writeln!(f, "borderless_fullscreen: {}", self.borderless_fullscreen)?;
        writeln!(f, "force_integer_scaling: {}", self.force_integer_scaling)?;
        writeln!(f, "window_width: {}", self.window_width)?;
        writeln!(f, "window_height: {}", self.window_height)?;
        writeln!(
            f,
            "audio_debugging_enabled: {}",
            self.audio_debugging_enabled
        )?;
        writeln!(f, "audio_60hz: {}", self.audio_60hz)?;
        writeln!(f, "color_scheme: {}", self.color_scheme)?;
        writeln!(f, "input_config: {}", self.input_config)?;
        writeln!(f, "hotkey_config: {}", self.hotkey_config)?;
        writeln!(f, "controller_config: {}", self.controller_config)?;

        Ok(())
    }
}
