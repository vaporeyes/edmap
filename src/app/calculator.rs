// ABOUTME: Floating calculator window — expression evaluation with memory + history.
// ABOUTME: Tiny recursive-descent parser supports + - * / % ^ parens and sqrt().

use eframe::egui::{self, Align2, Color32, RichText, Sense};

use super::state::EditorState;
use crate::theme;

/// Persistent state for the calculator. Lives on EditorState.
#[derive(Debug, Clone)]
pub struct CalculatorState {
    pub input: String,
    pub display: String,
    pub last_result: Option<f64>,
    pub memory: f64,
    /// Up to MAX_HISTORY most recent (expression, result) pairs, newest last.
    pub history: Vec<(String, f64)>,
    pub error: Option<String>,
}

const MAX_HISTORY: usize = 8;

impl Default for CalculatorState {
    fn default() -> Self {
        Self {
            input: String::new(),
            display: "0".into(),
            last_result: None,
            memory: 0.0,
            history: Vec::new(),
            error: None,
        }
    }
}

impl CalculatorState {
    fn evaluate(&mut self) {
        let expr = self.input.trim().to_string();
        if expr.is_empty() {
            return;
        }
        match parse_and_eval(&expr, self.last_result.unwrap_or(0.0), self.memory) {
            Ok(v) => {
                self.display = format_result(v);
                self.last_result = Some(v);
                self.error = None;
                self.history.push((expr, v));
                while self.history.len() > MAX_HISTORY {
                    self.history.remove(0);
                }
                self.input.clear();
            }
            Err(e) => {
                self.error = Some(e);
            }
        }
    }

    fn append(&mut self, s: &str) {
        self.input.push_str(s);
    }

    fn backspace(&mut self) {
        self.input.pop();
    }

    fn clear_entry(&mut self) {
        self.input.clear();
        self.error = None;
    }

    fn clear_all(&mut self) {
        *self = Self::default();
    }

    fn mem_add(&mut self) {
        if let Some(v) = self.last_result {
            self.memory += v;
        }
    }

    fn mem_sub(&mut self) {
        if let Some(v) = self.last_result {
            self.memory -= v;
        }
    }

    fn mem_recall(&mut self) {
        let m = self.memory;
        self.input.push_str(&format_result(m));
    }

    fn mem_clear(&mut self) {
        self.memory = 0.0;
    }
}

fn format_result(v: f64) -> String {
    if v.fract().abs() < 1e-9 && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

/// Toggle the calculator window from the editor.
pub fn toggle(state: &mut EditorState) {
    state.calculator_open = !state.calculator_open;
    if state.calculator_open && state.calculator.is_none() {
        state.calculator = Some(CalculatorState::default());
    }
}

/// Render the calculator window (non-modal, draggable).
pub fn draw(ctx: &egui::Context, state: &mut EditorState) {
    if !state.calculator_open {
        return;
    }
    if state.calculator.is_none() {
        state.calculator = Some(CalculatorState::default());
    }

    // Capture keyboard events BEFORE we open the window. Calling
    // `ctx.input(...)` from inside the Window closure can deadlock egui's
    // input lock; capturing once up front avoids that. Also check whether
    // some other widget owns keyboard focus so we don't double-handle keys.
    let wants_keyboard_elsewhere = ctx.wants_keyboard_input()
        && ctx.memory(|m| m.focused().is_some());
    let captured_events: Vec<egui::Event> = ctx.input(|i| i.events.clone());

    let mut keep_open = true;
    let calc = state.calculator.as_mut().unwrap();

    let screen = ctx.screen_rect();
    let max_h = (screen.height() * 0.85).min(440.0).max(280.0);

    egui::Window::new("Calculator")
        .frame(
            egui::Frame::none()
                .fill(theme::MENU_BG)
                .stroke(egui::Stroke::new(1.0, theme::VGA_BLACK))
                .inner_margin(egui::Margin::same(2.0)),
        )
        .open(&mut keep_open)
        .resizable(false)
        .default_width(248.0)
        .max_height(max_h)
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(2.0, 2.0);
            ui.set_max_width(244.0);

            // Display: large result on top, current input below.
            egui::Frame::none()
                .fill(theme::VIEWPORT_BG)
                .stroke(egui::Stroke::new(1.0, theme::VGA_GRAY))
                .inner_margin(egui::Margin::same(4.0))
                .show(ui, |ui| {
                    ui.set_min_width(232.0);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(&calc.display)
                                .color(theme::VGA_BRIGHT_GREEN)
                                .size(16.0)
                                .strong(),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let shown = if calc.input.is_empty() { " " } else { calc.input.as_str() };
                        ui.label(
                            RichText::new(shown)
                                .color(theme::VGA_BRIGHT_CYAN)
                                .size(11.0)
                                .monospace(),
                        );
                    });
                    if let Some(err) = &calc.error {
                        ui.label(RichText::new(err).color(theme::VGA_BRIGHT_RED).size(10.0));
                    } else if calc.memory != 0.0 {
                        ui.label(
                            RichText::new(format!("M = {}", format_result(calc.memory)))
                                .color(theme::VGA_YELLOW)
                                .size(10.0),
                        );
                    }
                });

            // Memory + clear row.
            ui.horizontal(|ui| {
                if calc_button(ui, "MC", 38.0).clicked() { calc.mem_clear(); }
                if calc_button(ui, "MR", 38.0).clicked() { calc.mem_recall(); }
                if calc_button(ui, "M+", 38.0).clicked() { calc.mem_add(); }
                if calc_button(ui, "M-", 38.0).clicked() { calc.mem_sub(); }
                if calc_button(ui, "C",  38.0).clicked() { calc.clear_all(); }
                if calc_button(ui, "←",  38.0).clicked() { calc.backspace(); }
            });
            // Function row.
            ui.horizontal(|ui| {
                if calc_button(ui, "(",    38.0).clicked() { calc.append("("); }
                if calc_button(ui, ")",    38.0).clicked() { calc.append(")"); }
                if calc_button(ui, "√",    38.0).clicked() { calc.append("sqrt("); }
                if calc_button(ui, "^",    38.0).clicked() { calc.append("^"); }
                if calc_button(ui, "%",    38.0).clicked() { calc.append("%"); }
                if calc_button(ui, "ans",  38.0).clicked() { calc.append("ans"); }
            });

            // 4×4 number/op grid.
            let rows = [
                ["7", "8", "9", "/"],
                ["4", "5", "6", "*"],
                ["1", "2", "3", "-"],
                ["0", ".", "=", "+"],
            ];
            for row in rows.iter() {
                ui.horizontal(|ui| {
                    for &btn in row.iter() {
                        let w = 58.0;
                        if calc_button(ui, btn, w).clicked() {
                            match btn {
                                "=" => calc.evaluate(),
                                _ => calc.append(btn),
                            }
                        }
                    }
                });
            }

            // History — collapsible to keep the window short. Default closed.
            if !calc.history.is_empty() {
                ui.add_space(2.0);
                egui::CollapsingHeader::new(
                    RichText::new(format!("History ({})", calc.history.len()))
                        .color(theme::MENU_FG),
                )
                .default_open(false)
                .show(ui, |ui| {
                    egui::ScrollArea::vertical().max_height(80.0).show(ui, |ui| {
                        for (expr, val) in calc.history.iter().rev() {
                            ui.label(
                                RichText::new(format!("{} = {}", expr, format_result(*val)))
                                    .color(theme::VGA_WHITE)
                                    .monospace()
                                    .size(10.0),
                            );
                        }
                    });
                });
            }

            // Keyboard input — drive from the events captured BEFORE the
            // window opened. Skip if some other widget owns keyboard focus.
            if !wants_keyboard_elsewhere {
                for ev in &captured_events {
                    match ev {
                        egui::Event::Text(t) => {
                            for c in t.chars() {
                                match c {
                                    '0'..='9' | '.' | '+' | '-' | '*' | '/' | '%' | '^' | '(' | ')' => {
                                        calc.input.push(c);
                                    }
                                    '=' => calc.evaluate(),
                                    _ => {}
                                }
                            }
                        }
                        egui::Event::Key { key: egui::Key::Enter, pressed: true, .. } => {
                            calc.evaluate();
                        }
                        egui::Event::Key { key: egui::Key::Backspace, pressed: true, .. } => {
                            calc.backspace();
                        }
                        egui::Event::Key { key: egui::Key::Delete, pressed: true, .. } => {
                            calc.clear_entry();
                        }
                        _ => {}
                    }
                }
            }

            let _ = Sense::click(); // silence unused import warning
            let _ = Color32::TRANSPARENT;
            let _ = Align2::CENTER_CENTER;
        });

    if !keep_open {
        state.calculator_open = false;
    }
}

fn calc_button(ui: &mut egui::Ui, label: &str, width: f32) -> egui::Response {
    let desired = egui::vec2(width, 20.0);
    let (rect, resp) = ui.allocate_exact_size(desired, Sense::click());
    let painter = ui.painter_at(rect);
    let hovered = resp.hovered();
    let pressed = resp.is_pointer_button_down_on();
    let bg = if hovered { theme::MENU_HILITE_BG } else { theme::MENU_BG };
    let fg = if hovered { theme::MENU_HILITE_FG } else { theme::MENU_FG };
    painter.rect_filled(rect, 0.0, bg);
    theme::draw_bevel(&painter, rect, pressed);
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        egui::FontId::new(13.0, egui::FontFamily::Monospace),
        fg,
    );
    resp
}

// ---------------- Expression parser (recursive descent) ----------------

fn parse_and_eval(src: &str, ans: f64, memory: f64) -> Result<f64, String> {
    let mut p = Parser::new(src, ans, memory);
    let v = p.parse_expr()?;
    p.skip_ws();
    if p.pos < p.bytes.len() {
        return Err(format!("Unexpected trailing input at column {}", p.pos + 1));
    }
    Ok(v)
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
    ans: f64,
    memory: f64,
}

impl<'a> Parser<'a> {
    fn new(src: &'a str, ans: f64, memory: f64) -> Self {
        Self { bytes: src.as_bytes(), pos: 0, ans, memory }
    }

    fn skip_ws(&mut self) {
        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    /// expr := term (('+' | '-') term)*
    fn parse_expr(&mut self) -> Result<f64, String> {
        let mut v = self.parse_term()?;
        loop {
            self.skip_ws();
            match self.peek() {
                Some(b'+') => { self.pos += 1; v += self.parse_term()?; }
                Some(b'-') => { self.pos += 1; v -= self.parse_term()?; }
                _ => break,
            }
        }
        Ok(v)
    }

    /// term := factor (('*' | '/' | '%') factor)*
    fn parse_term(&mut self) -> Result<f64, String> {
        let mut v = self.parse_factor()?;
        loop {
            self.skip_ws();
            match self.peek() {
                Some(b'*') => {
                    self.pos += 1;
                    v *= self.parse_factor()?;
                }
                Some(b'/') => {
                    self.pos += 1;
                    let r = self.parse_factor()?;
                    if r == 0.0 { return Err("Division by zero".into()); }
                    v /= r;
                }
                Some(b'%') => {
                    self.pos += 1;
                    let r = self.parse_factor()?;
                    if r == 0.0 { return Err("Modulo by zero".into()); }
                    v %= r;
                }
                _ => break,
            }
        }
        Ok(v)
    }

    /// factor := unary ('^' factor)?    -- right-associative
    fn parse_factor(&mut self) -> Result<f64, String> {
        let v = self.parse_unary()?;
        self.skip_ws();
        if let Some(b'^') = self.peek() {
            self.pos += 1;
            let exp = self.parse_factor()?;
            return Ok(v.powf(exp));
        }
        Ok(v)
    }

    /// unary := ('+' | '-') unary | primary
    fn parse_unary(&mut self) -> Result<f64, String> {
        self.skip_ws();
        match self.peek() {
            Some(b'+') => { self.pos += 1; self.parse_unary() }
            Some(b'-') => { self.pos += 1; Ok(-self.parse_unary()?) }
            _ => self.parse_primary(),
        }
    }

    /// primary := number | '(' expr ')' | identifier ('(' expr ')')?
    fn parse_primary(&mut self) -> Result<f64, String> {
        self.skip_ws();
        match self.peek() {
            Some(c) if c.is_ascii_digit() || c == b'.' => self.parse_number(),
            Some(b'(') => {
                self.pos += 1;
                let v = self.parse_expr()?;
                self.skip_ws();
                if self.peek() != Some(b')') {
                    return Err("Expected `)`".into());
                }
                self.pos += 1;
                Ok(v)
            }
            Some(c) if c.is_ascii_alphabetic() => {
                let start = self.pos;
                while self.peek().map(|b| b.is_ascii_alphanumeric()).unwrap_or(false) {
                    self.pos += 1;
                }
                let name = std::str::from_utf8(&self.bytes[start..self.pos])
                    .unwrap_or("")
                    .to_lowercase();
                self.skip_ws();
                if self.peek() == Some(b'(') {
                    self.pos += 1;
                    let arg = self.parse_expr()?;
                    self.skip_ws();
                    if self.peek() != Some(b')') {
                        return Err("Expected `)` after function argument".into());
                    }
                    self.pos += 1;
                    match name.as_str() {
                        "sqrt" => Ok(arg.sqrt()),
                        "abs"  => Ok(arg.abs()),
                        "sin"  => Ok(arg.to_radians().sin()),
                        "cos"  => Ok(arg.to_radians().cos()),
                        "tan"  => Ok(arg.to_radians().tan()),
                        "ln"   => Ok(arg.ln()),
                        "log"  => Ok(arg.log10()),
                        other  => Err(format!("Unknown function `{other}`")),
                    }
                } else {
                    match name.as_str() {
                        "ans"    => Ok(self.ans),
                        "m"      => Ok(self.memory),
                        "pi"     => Ok(std::f64::consts::PI),
                        "e"      => Ok(std::f64::consts::E),
                        other    => Err(format!("Unknown identifier `{other}`")),
                    }
                }
            }
            None => Err("Unexpected end of expression".into()),
            Some(c) => Err(format!("Unexpected character `{}`", c as char)),
        }
    }

    fn parse_number(&mut self) -> Result<f64, String> {
        let start = self.pos;
        let mut saw_dot = false;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                self.pos += 1;
            } else if c == b'.' && !saw_dot {
                saw_dot = true;
                self.pos += 1;
            } else {
                break;
            }
        }
        let raw = std::str::from_utf8(&self.bytes[start..self.pos])
            .map_err(|_| "Invalid number".to_string())?;
        raw.parse::<f64>().map_err(|_| format!("Invalid number `{raw}`"))
    }
}

#[cfg(test)]
mod tests {
    use super::parse_and_eval;
    fn ev(s: &str) -> f64 {
        parse_and_eval(s, 0.0, 0.0).unwrap()
    }

    #[test]
    fn arithmetic_precedence() {
        assert!((ev("2 + 3 * 4") - 14.0).abs() < 1e-9);
        assert!((ev("(2 + 3) * 4") - 20.0).abs() < 1e-9);
        assert!((ev("2 ^ 10") - 1024.0).abs() < 1e-9);
        assert!((ev("17 % 5") - 2.0).abs() < 1e-9);
        assert!((ev("-3 ^ 2") - 9.0).abs() < 1e-9); // unary binds tighter than ^? actually -3^2 → -(3^2) in math; we do (-3)^2 = 9 because we eval unary before factor. Either is defensible.
    }

    #[test]
    fn functions() {
        assert!((ev("sqrt(144)") - 12.0).abs() < 1e-9);
        assert!((ev("abs(-7)") - 7.0).abs() < 1e-9);
        assert!((ev("pi") - std::f64::consts::PI).abs() < 1e-9);
    }

    #[test]
    fn divide_by_zero_errors() {
        assert!(parse_and_eval("1/0", 0.0, 0.0).is_err());
    }

    #[test]
    fn unmatched_paren_errors() {
        assert!(parse_and_eval("(1 + 2", 0.0, 0.0).is_err());
    }
}
