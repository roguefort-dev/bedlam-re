//! Original shop input order and debounce, EXW 0x441257..0x44337c.
use super::{
    catalog::{category_at, CATEGORIES},
    controls::Control,
    transactions::Transactions,
};
use bedlam_core::input::InputFrame;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Outcome {
    #[default]
    None,
    Done,
    AutoRequested,
}

/// One fixed shop-frame input state. The owning scene handles Auto before
/// the next frame and transfers the loadout only after Done.
pub struct ArmouryInput {
    state: Transactions,
    category: Option<usize>,
    cursor: (i32, i32),
    debounce: u8,
    panel_age: u32,
    weapon_ages: [u8; 7],
    equipment_ages: [u8; 2],
    replaced_equipment: Option<usize>,
}
impl ArmouryInput {
    pub fn new(state: Transactions) -> Self {
        Self {
            state,
            category: None,
            cursor: (320, 240),
            debounce: 0,
            panel_age: 0,
            weapon_ages: [12; 7],
            equipment_ages: [9; 2],
            replaced_equipment: None,
        }
    }
    pub fn state(&self) -> &Transactions {
        &self.state
    }
    pub fn cursor(&self) -> (i32, i32) {
        self.cursor
    }
    pub fn category(&self) -> Option<usize> {
        self.category
    }
    pub fn ready(&self) -> bool {
        self.state
            .weapons()
            .iter()
            .zip(self.weapon_ages)
            .all(|(row, age)| row.is_none() || age >= 9)
    }
    pub fn draw(&self, renderer: &mut super::render::ArmouryRenderer, held: bool) {
        renderer.draw_frame(
            &self.state,
            self.category,
            &self.weapon_ages,
            &self.equipment_ages,
            self.panel_age,
        );
        renderer.highlight(self.cursor, held, self.ready());
    }
    pub fn tick(&mut self, input: &InputFrame) -> Outcome {
        self.cursor.0 = (self.cursor.0 + i32::from(input.mouse_dx)).clamp(9, 631);
        self.cursor.1 = (self.cursor.1 + i32::from(input.mouse_dy)).clamp(9, 463);
        self.debounce = self.debounce.saturating_sub(1);
        self.replaced_equipment = None;
        let old_weapons = *self.state.weapons();
        let old_equipment = *self.state.equipment();
        let outcome = if input.mouse_buttons & 1 != 0 && self.debounce == 0 {
            self.click()
        } else {
            Outcome::None
        };
        for (slot, old) in old_weapons.iter().enumerate() {
            self.weapon_ages[slot] = if *old != self.state.weapons()[slot] {
                0
            } else {
                self.weapon_ages[slot].saturating_add(1).min(12)
            };
        }
        for (slot, old) in old_equipment.iter().enumerate() {
            self.equipment_ages[slot] =
                if *old != self.state.equipment()[slot] || self.replaced_equipment == Some(slot) {
                    0
                } else {
                    self.equipment_ages[slot].saturating_add(1).min(9)
                };
        }
        self.panel_age = self.panel_age.saturating_add(1);
        outcome
    }
    fn click(&mut self) -> Outcome {
        let (x, y) = self.cursor;
        match Control::at(self.cursor) {
            Some(Control::Auto) if self.state.cart().is_none() => return Outcome::AutoRequested,
            Some(Control::Increase) => {
                if self.state.increase() {
                    self.debounce = 8;
                }
                return Outcome::None;
            }
            Some(Control::Decrease) => {
                if self.state.decrease() {
                    self.debounce = 8;
                }
                return Outcome::None;
            }
            Some(Control::Buy) if self.state.cart().is_some() => {
                let cart = self.state.cart().expect("selected cart");
                if self.state.buy() {
                    self.debounce = 10;
                    if cart.category == 8 {
                        self.replaced_equipment = self
                            .state
                            .equipment()
                            .iter()
                            .position(|row| row.is_some_and(|r| r.item == cart.item));
                    }
                }
                return Outcome::None;
            }
            Some(Control::Cancel) if self.state.cart().is_some() => {
                self.state.cancel();
                return Outcome::None;
            }
            Some(Control::Done) if self.ready() && self.state.has_weapon() => return Outcome::Done,
            _ => {}
        }
        if (535..=636).contains(&x) && (340..=411).contains(&y) {
            let slot = ((y - 340) / 10) as usize;
            if self.state.sell_weapon(slot) {
                self.category = self.state.cart().map(|c| c.category);
                return Outcome::None;
            }
        }
        if (544..=636).contains(&x)
            && (416..=435).contains(&y)
            && self.state.sell_equipment(((y - 416) / 10) as usize)
        {
            self.category = self.state.cart().map(|c| c.category);
            return Outcome::None;
        }
        if let Some((category, item)) = self
            .category
            .and_then(|c| CATEGORIES[c].item_at(self.cursor).map(|i| (c, i)))
        {
            if self.state.select(category, item) {
                self.debounce = 3;
                return Outcome::None;
            }
        }
        if let Some(category) = category_at(self.cursor) {
            if self.category != Some(category) {
                self.category = Some(category);
                self.panel_age = 0;
                self.debounce = 10;
                self.state.cancel();
            }
        }
        Outcome::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::armoury::catalog::{Catalog, Mode};
    fn input() -> ArmouryInput {
        ArmouryInput::new(Transactions::new(
            Catalog::new(Mode::Campaign, 1, [0; 15]).unwrap(),
            3500,
        ))
    }
    fn click(s: &mut ArmouryInput, x: i32, y: i32) -> Outcome {
        let (cx, cy) = s.cursor();
        s.tick(&InputFrame {
            mouse_dx: (x - cx) as i16,
            mouse_dy: (y - cy) as i16,
            mouse_buttons: 1,
            ..Default::default()
        })
    }
    fn wait(s: &mut ArmouryInput, n: usize) {
        for _ in 0..n {
            s.tick(&InputFrame::default());
        }
    }
    #[test]
    fn real_pointer_purchase_and_done_require_owned_weapon() {
        let mut s = input();
        assert_eq!(click(&mut s, 590, 455), Outcome::None);
        click(&mut s, 237, 97);
        wait(&mut s, 10);
        click(&mut s, 200, 105);
        wait(&mut s, 3);
        assert_eq!(s.state().cash(), 3400);
        click(&mut s, 500, 350);
        assert_eq!(s.state().weapons()[0].unwrap().name, 2);
        assert!(!s.ready());
        assert_eq!(click(&mut s, 590, 455), Outcome::None);
        wait(&mut s, 10);
        assert_eq!(click(&mut s, 590, 455), Outcome::Done);
    }
    #[test]
    fn held_quantity_repeats_after_original_debounce() {
        let mut s = input();
        click(&mut s, 237, 97);
        wait(&mut s, 10);
        click(&mut s, 200, 105);
        wait(&mut s, 3);
        click(&mut s, 628, 320);
        assert_eq!(s.state().cart().unwrap().amount, 600);
        for _ in 0..7 {
            click(&mut s, 628, 320);
        }
        assert_eq!(s.state().cart().unwrap().amount, 600);
        click(&mut s, 628, 320);
        assert_eq!(s.state().cart().unwrap().amount, 900);
    }
}
