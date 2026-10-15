//! Player input for one simulation step.

/// Buttons held during one sim step. The sim works out "just pressed" itself
/// by comparing with the previous step, so a replay only has to store what is
/// held each step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct InputFrame {
    pub left: bool,
    pub right: bool,
    pub jump: bool,
}

impl InputFrame {
    pub const NONE: InputFrame = InputFrame {
        left: false,
        right: false,
        jump: false,
    };

    /// -1 (left), 0, or +1 (right). Both held cancel out.
    pub fn axis(self) -> f32 {
        (self.right as i8 - self.left as i8) as f32
    }

    /// Replay notation: letters `L`, `R`, `J` in that order, or `-` for none.
    pub fn to_code(self) -> String {
        let mut s = String::new();
        if self.left {
            s.push('L');
        }
        if self.right {
            s.push('R');
        }
        if self.jump {
            s.push('J');
        }
        if s.is_empty() {
            s.push('-');
        }
        s
    }

    /// Parses replay notation (any order, case-insensitive).
    pub fn from_code(code: &str) -> Option<InputFrame> {
        if code == "-" {
            return Some(InputFrame::NONE);
        }
        let mut f = InputFrame::NONE;
        for c in code.chars() {
            match c.to_ascii_uppercase() {
                'L' => f.left = true,
                'R' => f.right = true,
                'J' => f.jump = true,
                _ => return None,
            }
        }
        if code.is_empty() { None } else { Some(f) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_round_trip() {
        for bits in 0..8u8 {
            let f = InputFrame {
                left: bits & 1 != 0,
                right: bits & 2 != 0,
                jump: bits & 4 != 0,
            };
            assert_eq!(InputFrame::from_code(&f.to_code()), Some(f));
        }
    }

    #[test]
    fn code_parsing() {
        assert_eq!(InputFrame::from_code("jr").unwrap().to_code(), "RJ");
        assert_eq!(InputFrame::from_code("X"), None);
        assert_eq!(InputFrame::from_code(""), None);
    }

    #[test]
    fn axis() {
        assert_eq!(InputFrame::from_code("L").unwrap().axis(), -1.0);
        assert_eq!(InputFrame::from_code("LR").unwrap().axis(), 0.0);
    }
}
