//! EXW shop button rectangles and CONLITE indices, 0x4435d3..0x4437b1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Control {
    Buy,
    Cancel,
    Auto,
    Increase,
    Decrease,
    Done,
}
impl Control {
    pub fn at((x, y): (i32, i32)) -> Option<Self> {
        [
            Self::Buy,
            Self::Cancel,
            Self::Auto,
            Self::Increase,
            Self::Decrease,
            Self::Done,
        ]
        .into_iter()
        .find(|c| {
            let (left, top, right, bottom) = c.rect();
            (left..=right).contains(&x) && (top..=bottom).contains(&y)
        })
    }
    fn rect(self) -> (i32, i32, i32, i32) {
        match self {
            Self::Buy => (482, 340, 525, 357),
            Self::Cancel => (482, 362, 525, 379),
            Self::Auto => (482, 393, 525, 410),
            Self::Increase => (625, 316, 634, 333),
            Self::Decrease => (481, 316, 490, 333),
            Self::Done => (568, 446, 615, 472),
        }
    }
    pub(super) fn image(self) -> (usize, i32, i32) {
        match self {
            Self::Buy => (0, 479, 337),
            Self::Cancel => (1, 479, 361),
            Self::Auto => (2, 480, 391),
            Self::Increase => (3, 623, 314),
            Self::Decrease => (4, 479, 314),
            Self::Done => (5, 568, 446),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_button_edges_and_gaps() {
        assert_eq!(Control::at((482, 340)), Some(Control::Buy));
        assert_eq!(Control::at((525, 357)), Some(Control::Buy));
        assert_eq!(Control::at((525, 358)), None);
        assert_eq!(Control::at((625, 320)), Some(Control::Increase));
        assert_eq!(Control::at((490, 333)), Some(Control::Decrease));
        assert_eq!(Control::at((615, 472)), Some(Control::Done));
        assert_eq!(Control::at((616, 472)), None);
    }
}
