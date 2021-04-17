use core::sync::atomic::{AtomicBool, Ordering};

use spin::Mutex;

use alloc::prelude::v1::*;

pub struct KeyboardDriver {
    keycode_buffer: [u8; 8],
    keypress_buffer: Vec<KeyEvent>,
    driver_state: DriverState,
    key_state: [bool; 256],
    active_layout: KeyboardLayout,
}

impl KeyboardDriver {
    pub const fn new() -> Self {
        Self {
            keycode_buffer: [0; 8],
            keypress_buffer: vec![],
            driver_state: DriverState::Idle,
            key_state: [false; 256],
            active_layout: {
                let mut layout = KeyboardLayout::empty();
                generate_swedish_layout(&mut layout);
                layout
            },
        }
    }

    fn handle_message(&mut self, message: u8) {
        match self.driver_state {
            DriverState::Idle => {
                self.keycode_buffer[0] = message;
                if message >= 0xA0 {
                    self.driver_state = DriverState::WaitingForExtended(1);
                } else {
                    self.keycode_buffer[0] = message;
                    let key_press = self.handle_scancode(1);
                    self.keypress_buffer.push(key_press);
                    HAS_KEY_IN_BUFFER.store(true, Ordering::Release);
                }
            }
            DriverState::WaitingForExtended(s) => {
                if message < 0xA0 {
                    match &self.keycode_buffer[0..s] {
                        [0xE0] if message == 0x12 => {
                            self.keycode_buffer[1] = message;
                            self.driver_state = DriverState::WaitingForExtended(2);
                        }
                        [0xE0, 0xF0] if message == 0x7C => {
                            self.keycode_buffer[2] = message;
                            self.driver_state = DriverState::WaitingForExtended(3);
                        }
                        [0xE1] if message == 0x14 => {
                            self.keycode_buffer[1] = message;
                            self.driver_state = DriverState::WaitingForExtended(2);
                        }
                        [0xE1, 0x14] if message == 0x77 => {
                            self.keycode_buffer[2] = message;
                            self.driver_state = DriverState::WaitingForExtended(3);
                        }
                        [0xE1, 0x14, 0x77, 0xE1, 0xF0] if message == 0x14 => {
                            self.keycode_buffer[5] = message;
                            self.driver_state = DriverState::WaitingForExtended(6);
                        }
                        _ => {
                            self.keycode_buffer[s] = message;
                            let key_press = self.handle_scancode(s + 1);
                            self.keypress_buffer.push(key_press);
                            HAS_KEY_IN_BUFFER.store(true, Ordering::Release);
                            self.driver_state = DriverState::Idle;
                        }
                    }
                } else {
                    self.keycode_buffer[s] = message;
                    self.driver_state = DriverState::WaitingForExtended(s + 1);
                }
            }
        }
    }

    fn handle_scancode(&mut self, len: usize) -> KeyEvent {
        let mut buffer = self.keycode_buffer.clone();
        let mut code = &mut buffer[..len];
        let released = match code {
            [_] => false,
            [0xF0, b] => {
                code[0] = *b;
                code = &mut code[..1];
                true
            }
            [0xE0, _] => false,
            [0xE0, 0xF0, b] => {
                code[1] = *b;
                code = &mut code[..2];
                true
            }
            [0xE0, 0xF0, 0x7C, 0xE0, 0xF0, 0x12] => {
                code[1] = 0x12;
                code[2] = 0xE0;
                code[3] = 0x7C;
                code = &mut code[..4];
                true
            }
            // Pause has no release state
            [0xE1, 0x14, 0x77, 0xE1, 0xF0, 0x14, 0xF0, 0x77] => false,
            _ => panic!(
                "Internal bug: unimplemented scan code {:x?} should have been caught earlier",
                code
            ),
        };

        let keycode = self.translate_scancode(code);

        let state = if released {
            KeyState::Released
        } else if self.key_state[keycode as usize] {
            KeyState::Held
        } else {
            KeyState::Pressed
        };

        self.key_state[keycode as usize] = !released;
        let shift =
            self.key_state[KeyCode::LShift as usize] || self.key_state[KeyCode::RShift as usize];
        let control = self.key_state[KeyCode::LControl as usize]
            || self.key_state[KeyCode::RControl as usize];
        let alt = self.key_state[KeyCode::LAlt as usize];
        let altgr = self.key_state[KeyCode::RAlt as usize];
        let meta =
            self.key_state[KeyCode::LMeta as usize] || self.key_state[KeyCode::RMeta as usize];

        let modifiers = Modifiers {
            shift,
            alt,
            altgr,
            control,
            meta,
        };
        let char = if !released {
            self.active_layout.transform(keycode, modifiers)
        } else {
            None
        };

        KeyEvent {
            key_code: keycode,
            key_state: state,
            modifiers,
            char,
        }
    }

    /// See the `keycodes.txt` file
    /// and [the osdev wiki](https://wiki.osdev.org/PS/2_Keyboard#Scan_Code_Set_2).
    fn translate_scancode(&self, codes: &[u8]) -> KeyCode {
        use KeyCode::*;
        match codes {
            [0x01] => F9,
            [0x03] => F5,
            [0x04] => F3,
            [0x05] => F1,
            [0x06] => F2,
            [0x07] => F12,
            [0x09] => F10,
            [0x0A] => F8,
            [0x0B] => F6,
            [0x0C] => F4,
            [0x0D] => Tab,
            [0x0E] => Backtick,
            [0x11] => LAlt,
            [0x12] => LShift,
            [0x14] => LControl,
            [0x15] => Q,
            [0x16] => Digit1,
            [0x1A] => Z,
            [0x1B] => S,
            [0x1C] => A,
            [0x1D] => W,
            [0x1E] => Digit2,
            [0x21] => C,
            [0x22] => X,
            [0x23] => D,
            [0x24] => E,
            [0x25] => Digit4,
            [0x26] => Digit3,
            [0x29] => Space,
            [0x2A] => V,
            [0x2B] => F,
            [0x2C] => T,
            [0x2D] => R,
            [0x2E] => Digit5,
            [0x31] => N,
            [0x32] => B,
            [0x33] => H,
            [0x34] => G,
            [0x35] => Y,
            [0x36] => Digit6,
            [0x3A] => M,
            [0x3B] => J,
            [0x3C] => U,
            [0x3D] => Digit7,
            [0x3E] => Digit8,
            [0x41] => Comma,
            [0x42] => K,
            [0x43] => I,
            [0x44] => O,
            [0x45] => Digit0,
            [0x46] => Digit9,
            [0x49] => Period,
            [0x4A] => Slash,
            [0x4B] => L,
            [0x4C] => Semicolon,
            [0x4D] => P,
            [0x4E] => Minus,
            [0x52] => Quote,
            [0x54] => LBracket,
            [0x55] => Equal,
            [0x58] => CapsLock,
            [0x59] => RShift,
            [0x5A] => Enter,
            [0x5B] => RBracket,
            [0x5D] => Backslash,
            [0x61] => Pipe,
            [0x66] => Backspace,
            [0x76] => Escape,
            [0x78] => F11,
            [0x83] => F7,

            [0xE0, 0x11] => RAlt,
            [0xE0, 0x14] => RControl,
            [0xE0, 0x1F] => LMeta,
            //[0xE0 0x2B] => Calculator,
            [0xE0, 0x6B] => Left,
            [0xE0, 0x6C] => Home,
            [0xE0, 0x69] => End,
            [0xE0, 0x70] => Insert,
            [0xE0, 0x71] => Delete,
            [0xE0, 0x72] => Down,
            [0xE0, 0x74] => Right,
            [0xE0, 0x75] => Up,
            [0xE0, 0x7A] => PageDown,
            [0xE0, 0x7D] => PageUp,

            _ => {
                println!("Keycode {:x?} not implemented", codes);
                Unknown
            }
        }
    }
}

enum DriverState {
    Idle,
    WaitingForExtended(usize),
}

static KEYBOARD_DRIVER: Mutex<KeyboardDriver> = Mutex::new(KeyboardDriver::new());
static HAS_KEY_IN_BUFFER: AtomicBool = AtomicBool::new(false);

pub(super) fn handle_message(message: u8) {
    KEYBOARD_DRIVER
        .try_lock()
        .expect(
            "PS/2 driver tried sending scancode part to handle while a character was getting read",
        )
        .handle_message(message);
}

/// Returns whenever a key is pressed, or repeated when held down.
/// Does not return release events.
pub fn get_key() -> KeyEvent {
    loop {
        while !HAS_KEY_IN_BUFFER.load(Ordering::Acquire) {}
        let mut driver = KEYBOARD_DRIVER.lock();
        let ret = driver.keypress_buffer.pop().unwrap();
        if driver.keypress_buffer.len() == 0 {
            HAS_KEY_IN_BUFFER.store(false, Ordering::Release);
        }
        if ret.key_state != KeyState::Released {
            break ret;
        }
    }
}

#[derive(Clone, Copy)]
pub struct KeyEvent {
    pub key_code: KeyCode,
    pub key_state: KeyState,
    pub modifiers: Modifiers,
    pub char: Option<char>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KeyCode {
    Escape = 0x00,
    F1 = 0x01,
    F2 = 0x02,
    F3 = 0x03,
    F4 = 0x04,
    F5 = 0x05,
    F6 = 0x06,
    F7 = 0x07,
    F8 = 0x08,
    F9 = 0x09,
    F10 = 0x0A,
    F11 = 0x0B,
    F12 = 0x0C,
    PrintScrn = 0x0D,
    ScrollLock = 0x0E,
    Break = 0x0F,

    Backtick = 0x20,
    Digit1 = 0x21,
    Digit2 = 0x22,
    Digit3 = 0x23,
    Digit4 = 0x24,
    Digit5 = 0x25,
    Digit6 = 0x26,
    Digit7 = 0x27,
    Digit8 = 0x28,
    Digit9 = 0x29,
    Digit0 = 0x2A,
    Minus = 0x2B,
    Equal = 0x2C,
    Backspace = 0x2D,
    Insert = 0x2E,
    Home = 0x2F,
    PageUp = 0x30,
    NumLock = 0x31,
    NumpadDivide = 0x32,
    NumpadMultiply = 0x33,
    NumpadSubtract = 0x34,

    Tab = 0x40,
    Q = 0x41,
    W = 0x42,
    E = 0x43,
    R = 0x44,
    T = 0x45,
    Y = 0x46,
    U = 0x47,
    I = 0x48,
    O = 0x49,
    P = 0x4A,
    LBracket = 0x4B,
    RBracket = 0x4C,
    Enter = 0x4D,
    Delete = 0x4E,
    End = 0x4F,
    PageDown = 0x50,
    Numpad7 = 0x51,
    Numpad8 = 0x52,
    Numpad9 = 0x53,
    NumpadAdd = 0x54,

    CapsLock = 0x60,
    A = 0x61,
    S = 0x62,
    D = 0x63,
    F = 0x64,
    G = 0x65,
    H = 0x66,
    J = 0x67,
    K = 0x68,
    L = 0x69,
    Semicolon = 0x6A,
    Quote = 0x6B,
    Backslash = 0x6C,
    Numpad4 = 0x6D,
    Numpad5 = 0x6E,
    Numpad6 = 0x6F,

    LShift = 0x80,
    Pipe = 0x81,
    Z = 0x82,
    X = 0x83,
    C = 0x84,
    V = 0x85,
    B = 0x86,
    N = 0x87,
    M = 0x88,
    Comma = 0x89,
    Period = 0x8A,
    Slash = 0x8B,
    RShift = 0x8C,
    Up = 0x8D,
    Numpad1 = 0x8E,
    Numpad2 = 0x8F,
    Numpad3 = 0x90,
    NumpadEnter = 0x91,

    LControl = 0xA0,
    LMeta = 0xA1,
    LAlt = 0xA2,
    Space = 0xA3,
    RAlt = 0xA4,
    RMeta = 0xA5,
    Menu = 0xA6,
    RControl = 0xA7,
    Left = 0xA8,
    Down = 0xA9,
    Right = 0xAA,
    Numpad0 = 0xAB,
    NumpadDecimal = 0xAC,

    // Misc: 0xC0..=0xFF (64 keycodes)
    Unknown = 0xFF,
}

#[derive(Clone, Copy)]
pub struct Modifiers {
    pub shift: bool,
    pub alt: bool,
    pub control: bool,
    pub meta: bool,
    pub altgr: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum KeyState {
    Pressed,
    Held,
    Released,
}

const MAX_KEYCODE_TRANSFORMATION_COUNT: usize = 4;

struct KeyboardLayout {
    transformations: [[Option<Transformation>; MAX_KEYCODE_TRANSFORMATION_COUNT]; 256],
}

impl KeyboardLayout {
    const fn empty() -> Self {
        Self {
            transformations: [[None; MAX_KEYCODE_TRANSFORMATION_COUNT]; 256],
        }
    }

    const fn add_transform(
        &mut self,
        keycode: KeyCode,
        modifier_filter: ModifierFilter,
        result: char,
    ) -> Result<(), ()> {
        let transforms = &mut self.transformations[keycode as usize];
        // for entry in transforms.as_mut().unwrap() {
        //     if entry.is_none() {
        //         entry.replace(Transformation::new(modifier_filter, result));
        //         return Ok(())
        //     }
        // }

        // This code would need to be updated every time MAX_KEYCODE_TRANSFORMATION_COUNT changes

        assert!(MAX_KEYCODE_TRANSFORMATION_COUNT == 4);
        let transforms = &mut self.transformations[keycode as usize];
        if transforms[0].is_none() {
            transforms[0] = Some(Transformation::new(modifier_filter, result));
            return Ok(());
        }
        if transforms[1].is_none() {
            transforms[1] = Some(Transformation::new(modifier_filter, result));
            return Ok(());
        }
        if transforms[2].is_none() {
            transforms[2] = Some(Transformation::new(modifier_filter, result));
            return Ok(());
        }
        if transforms[3].is_none() {
            transforms[3] = Some(Transformation::new(modifier_filter, result));
            return Ok(());
        }

        Err(())
    }

    fn transform(&self, keycode: KeyCode, modifiers: Modifiers) -> Option<char> {
        let transforms = &self.transformations[keycode as usize];
        let transform = transforms.iter().find(|t| {
            t.map(|t| t.modifier_filter.matches(modifiers))
                .unwrap_or_default()
        })?;
        let transform = (*transform)?;
        Some(transform.result)
    }
}

#[derive(Clone, Copy)]
struct Transformation {
    modifier_filter: ModifierFilter,
    result: char,
}

impl Transformation {
    const fn new(modifier_filter: ModifierFilter, result: char) -> Self {
        Self {
            modifier_filter,
            result,
        }
    }
}

#[derive(Clone, Copy)]
struct ModifierFilter {
    shift: Filter,
    control: Filter,
    alt: Filter,
    altgr: Filter,
    meta: Filter,
}

impl ModifierFilter {
    const SHIFT: Self = Self::new().shift();
    const CONTROL: Self = Self::new().shift();
    const ALT: Self = Self::new().shift();
    const ALTGR: Self = Self::new().shift();
    const META: Self = Self::new().shift();

    const fn new() -> Self {
        Self {
            shift: Filter::Released,
            control: Filter::Released,
            alt: Filter::Released,
            altgr: Filter::Released,
            meta: Filter::Released,
        }
    }

    const fn shift(mut self) -> Self {
        self.shift = Filter::Pressed;
        self
    }

    const fn control(mut self) -> Self {
        self.control = Filter::Pressed;
        self
    }

    const fn alt(mut self) -> Self {
        self.alt = Filter::Pressed;
        self
    }

    const fn altgr(mut self) -> Self {
        self.altgr = Filter::Pressed;
        self
    }

    const fn meta(mut self) -> Self {
        self.meta = Filter::Pressed;
        self
    }

    const fn matches(&self, modifiers: Modifiers) -> bool {
        self.shift.matches(modifiers.shift)
            && self.control.matches(modifiers.control)
            && self.alt.matches(modifiers.alt)
            && self.altgr.matches(modifiers.altgr)
            && self.meta.matches(modifiers.meta)
    }
}

#[derive(Clone, Copy)]
enum Filter {
    Pressed,
    Released,
    DontCare,
}

impl Filter {
    const fn matches(&self, modifier: bool) -> bool {
        match self {
            Self::Pressed => modifier,
            Self::Released => !modifier,
            Self::DontCare => true,
        }
    }
}

const fn generate_swedish_layout(layout: &mut KeyboardLayout) {
    use KeyCode::*;
    const NONE: ModifierFilter = ModifierFilter::new();
    const SHIFT: ModifierFilter = ModifierFilter::SHIFT;
    const ALTGR: ModifierFilter = ModifierFilter::ALTGR;

    unwrap(layout.add_transform(Backtick, NONE, '§'));
    unwrap(layout.add_transform(Digit1, NONE, '1'));
    unwrap(layout.add_transform(Digit2, NONE, '2'));
    unwrap(layout.add_transform(Digit3, NONE, '3'));
    unwrap(layout.add_transform(Digit4, NONE, '4'));
    unwrap(layout.add_transform(Digit5, NONE, '5'));
    unwrap(layout.add_transform(Digit6, NONE, '6'));
    unwrap(layout.add_transform(Digit7, NONE, '7'));
    unwrap(layout.add_transform(Digit8, NONE, '8'));
    unwrap(layout.add_transform(Digit9, NONE, '9'));
    unwrap(layout.add_transform(Digit0, NONE, '0'));
    unwrap(layout.add_transform(Minus, NONE, '+'));
    unwrap(layout.add_transform(Equal, NONE, '´'));
    unwrap(layout.add_transform(Q, NONE, 'q'));
    unwrap(layout.add_transform(W, NONE, 'w'));
    unwrap(layout.add_transform(E, NONE, 'e'));
    unwrap(layout.add_transform(R, NONE, 'r'));
    unwrap(layout.add_transform(T, NONE, 't'));
    unwrap(layout.add_transform(Y, NONE, 'y'));
    unwrap(layout.add_transform(U, NONE, 'u'));
    unwrap(layout.add_transform(I, NONE, 'i'));
    unwrap(layout.add_transform(O, NONE, 'o'));
    unwrap(layout.add_transform(P, NONE, 'p'));
    unwrap(layout.add_transform(LBracket, NONE, 'å'));
    unwrap(layout.add_transform(RBracket, NONE, '¨'));
    unwrap(layout.add_transform(A, NONE, 'a'));
    unwrap(layout.add_transform(S, NONE, 's'));
    unwrap(layout.add_transform(D, NONE, 'd'));
    unwrap(layout.add_transform(F, NONE, 'f'));
    unwrap(layout.add_transform(G, NONE, 'g'));
    unwrap(layout.add_transform(H, NONE, 'h'));
    unwrap(layout.add_transform(J, NONE, 'j'));
    unwrap(layout.add_transform(K, NONE, 'k'));
    unwrap(layout.add_transform(L, NONE, 'l'));
    unwrap(layout.add_transform(Semicolon, NONE, 'ö'));
    unwrap(layout.add_transform(Quote, NONE, 'ä'));
    unwrap(layout.add_transform(Backslash, NONE, '\''));
    unwrap(layout.add_transform(Pipe, NONE, '<'));
    unwrap(layout.add_transform(Z, NONE, 'z'));
    unwrap(layout.add_transform(X, NONE, 'x'));
    unwrap(layout.add_transform(C, NONE, 'c'));
    unwrap(layout.add_transform(V, NONE, 'v'));
    unwrap(layout.add_transform(B, NONE, 'b'));
    unwrap(layout.add_transform(N, NONE, 'n'));
    unwrap(layout.add_transform(M, NONE, 'm'));
    unwrap(layout.add_transform(Comma, NONE, ','));
    unwrap(layout.add_transform(Period, NONE, '.'));
    unwrap(layout.add_transform(Slash, NONE, '-'));
    unwrap(layout.add_transform(Space, NONE, ' '));

    unwrap(layout.add_transform(Backtick, SHIFT, '½'));
    unwrap(layout.add_transform(Digit1, SHIFT, '!'));
    unwrap(layout.add_transform(Digit2, SHIFT, '"'));
    unwrap(layout.add_transform(Digit3, SHIFT, '#'));
    unwrap(layout.add_transform(Digit4, SHIFT, '¤'));
    unwrap(layout.add_transform(Digit5, SHIFT, '%'));
    unwrap(layout.add_transform(Digit6, SHIFT, '&'));
    unwrap(layout.add_transform(Digit7, SHIFT, '/'));
    unwrap(layout.add_transform(Digit8, SHIFT, '('));
    unwrap(layout.add_transform(Digit9, SHIFT, ')'));
    unwrap(layout.add_transform(Digit0, SHIFT, '='));
    unwrap(layout.add_transform(Minus, SHIFT, '?'));
    unwrap(layout.add_transform(Equal, SHIFT, '`'));
    unwrap(layout.add_transform(Q, SHIFT, 'Q'));
    unwrap(layout.add_transform(W, SHIFT, 'W'));
    unwrap(layout.add_transform(E, SHIFT, 'E'));
    unwrap(layout.add_transform(R, SHIFT, 'R'));
    unwrap(layout.add_transform(T, SHIFT, 'T'));
    unwrap(layout.add_transform(Y, SHIFT, 'Y'));
    unwrap(layout.add_transform(U, SHIFT, 'U'));
    unwrap(layout.add_transform(I, SHIFT, 'I'));
    unwrap(layout.add_transform(O, SHIFT, 'O'));
    unwrap(layout.add_transform(P, SHIFT, 'P'));
    unwrap(layout.add_transform(LBracket, SHIFT, 'Å'));
    unwrap(layout.add_transform(RBracket, SHIFT, '^'));
    unwrap(layout.add_transform(A, SHIFT, 'A'));
    unwrap(layout.add_transform(S, SHIFT, 'S'));
    unwrap(layout.add_transform(D, SHIFT, 'D'));
    unwrap(layout.add_transform(F, SHIFT, 'F'));
    unwrap(layout.add_transform(G, SHIFT, 'G'));
    unwrap(layout.add_transform(H, SHIFT, 'H'));
    unwrap(layout.add_transform(J, SHIFT, 'J'));
    unwrap(layout.add_transform(K, SHIFT, 'K'));
    unwrap(layout.add_transform(L, SHIFT, 'L'));
    unwrap(layout.add_transform(Semicolon, SHIFT, 'Ö'));
    unwrap(layout.add_transform(Quote, SHIFT, 'Ä'));
    unwrap(layout.add_transform(Backslash, SHIFT, '*'));
    unwrap(layout.add_transform(Pipe, SHIFT, '>'));
    unwrap(layout.add_transform(Z, SHIFT, 'Z'));
    unwrap(layout.add_transform(X, SHIFT, 'X'));
    unwrap(layout.add_transform(C, SHIFT, 'C'));
    unwrap(layout.add_transform(V, SHIFT, 'V'));
    unwrap(layout.add_transform(B, SHIFT, 'B'));
    unwrap(layout.add_transform(N, SHIFT, 'N'));
    unwrap(layout.add_transform(M, SHIFT, 'M'));
    unwrap(layout.add_transform(Comma, SHIFT, ';'));
    unwrap(layout.add_transform(Period, SHIFT, ':'));
    unwrap(layout.add_transform(Slash, SHIFT, '_'));

    unwrap(layout.add_transform(Digit1, ALTGR, '@'));
    unwrap(layout.add_transform(Digit2, ALTGR, '£'));
    unwrap(layout.add_transform(Digit3, ALTGR, '$'));
    unwrap(layout.add_transform(Digit7, ALTGR, '{'));
    unwrap(layout.add_transform(Digit8, ALTGR, '['));
    unwrap(layout.add_transform(Digit9, ALTGR, ']'));
    unwrap(layout.add_transform(Digit0, ALTGR, '}'));
    unwrap(layout.add_transform(Minus, ALTGR, '\\'));
    unwrap(layout.add_transform(E, ALTGR, '€'));
    unwrap(layout.add_transform(RBracket, ALTGR, '~'));
    unwrap(layout.add_transform(Pipe, ALTGR, '|'));
}

use core::fmt::Debug;
const fn unwrap<T, E: Debug>(result: Result<T, E>) -> T {
    match result {
        Ok(v) => v,
        Err(e) => panic!("Tried to unwrap an Err value"),
    }
}
