pub struct ANSIColors;

impl ANSIColors {
    pub const green: &'static str = "\u{001B}[32m";
    pub const blue: &'static str = "\u{001B}[34m";
    pub const yellow: &'static str = "\u{001B}[33m";
    pub const magenta: &'static str = "\u{001B}[35m";
    pub const cyan: &'static str = "\u{001B}[36m";
    pub const reset: &'static str = "\u{001B}[0m";
}

pub fn colorize(text: &str, color: &str) -> String {
    format!("{}{}{}", color, text, ANSIColors::reset)
}