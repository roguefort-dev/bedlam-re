//! Manual transactions from EXW 0x441acf..0x443951.
use super::catalog::{Catalog, Item};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cart {
    pub category: usize,
    pub item: usize,
    pub amount: u32,
    pub spend: u32,
}

/// The original persisted row stores amount and refundable cost as words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Owned {
    pub category: usize,
    pub item: usize,
    pub name: u16,
    pub amount: u16,
    pub paid: u16,
}

#[derive(Clone, Debug)]
pub struct Transactions {
    catalog: Catalog,
    balance: u32,
    cart: Option<Cart>,
    weapons: [Option<Owned>; 7],
    equipment: [Option<Owned>; 2],
}

impl Transactions {
    pub fn new(catalog: Catalog, balance: u32) -> Self {
        Self {
            catalog,
            balance: balance.max(100),
            cart: None,
            weapons: [None; 7],
            equipment: [None; 2],
        }
    }

    /// Deployment consumes the first three chassis kinds without a refund.
    /// Cleared original row metadata is hidden behind the empty-row model.
    pub(crate) fn consume_equipment(&mut self, consumed: [bool; 2]) {
        for (row, consume) in self.equipment.iter_mut().zip(consumed) {
            if consume {
                *row = None;
            }
        }
    }

    /// EXW Auto transaction pass. The caller supplies secondary bounded
    /// random draws, each strictly below the requested bound.
    pub fn auto(&mut self, mut random: impl FnMut(u32) -> u32) -> ([u8; 7], [u8; 2]) {
        let mut label_ages = [7u8; 47];
        for row in self.weapons.iter().chain(self.equipment.iter()).flatten() {
            self.balance = self.balance.wrapping_add(u32::from(row.paid));
        }
        self.weapons.fill(None);
        self.equipment.fill(None);
        self.cart = None;
        if self.balance >= 2400 && self.catalog.available(8, 4) {
            self.select(8, 4);
            self.buy();
        }
        let attempts = random(5);
        assert!(attempts < 5, "bounded secondary RNG");
        let attempts = attempts as usize + 3;
        for attempt in 0..attempts {
            for _ in 0..50 {
                let category = random(9) as usize;
                assert!(category < 9, "bounded secondary RNG");
                let count = super::catalog::CATEGORIES[category].items.len();
                let index = random(count as u32) as usize;
                assert!(index < count, "bounded secondary RNG");
                let item = self.catalog.item(category, index).expect("bounded item");
                if !self.catalog.available(category, index) || self.slot(category, item).is_none() {
                    continue;
                }
                // A valid but unaffordable choice consumes this outer attempt.
                let name = item.name;
                if self.select(category, index) {
                    self.buy();
                    label_ages[name as usize] = 7 - attempt as u8;
                }
                break;
            }
        }
        if self.has_weapon() {
            loop {
                let mut unaffordable = false;
                for row in self.weapons[..attempts].iter_mut().flatten() {
                    let item = self
                        .catalog
                        .item(row.category, row.item)
                        .expect("owned item");
                    if self.balance < u32::from(item.price) {
                        unaffordable = true;
                    } else {
                        self.balance -= u32::from(item.price);
                        row.amount = row.amount.wrapping_add(item.amount);
                        row.paid = row.paid.wrapping_add(item.price);
                    }
                }
                if unaffordable {
                    break;
                }
            }
        }
        const RANK: [u8; 9] = [7, 2, 6, 4, 3, 1, 8, 5, 2];
        self.weapons
            .sort_by_key(|row| std::cmp::Reverse(row.map_or(0, |r| RANK[r.category])));
        self.cart = None;
        (
            self.weapons
                .map(|row| row.map_or(9, |r| label_ages[r.name as usize])),
            self.equipment
                .map(|row| row.map_or(9, |r| label_ages[r.name as usize])),
        )
    }

    pub fn catalog(&self) -> &Catalog {
        &self.catalog
    }
    pub fn balance(&self) -> u32 {
        self.balance
    }
    pub fn cash(&self) -> u32 {
        self.balance - self.cart.map_or(0, |cart| cart.spend)
    }
    pub fn cart(&self) -> Option<Cart> {
        self.cart
    }
    pub fn weapons(&self) -> &[Option<Owned>; 7] {
        &self.weapons
    }
    pub fn equipment(&self) -> &[Option<Owned>; 2] {
        &self.equipment
    }
    pub fn has_weapon(&self) -> bool {
        self.weapons.iter().any(Option::is_some)
    }

    fn slot(&self, category: usize, item: &Item) -> Option<usize> {
        if category != 8 {
            if self
                .weapons
                .iter()
                .flatten()
                .any(|row| row.name == item.name)
            {
                return None;
            }
            return self.weapons.iter().position(Option::is_none);
        }
        if let Some(slot) = self
            .equipment
            .iter()
            .position(|row| row.is_some_and(|r| r.name == item.name))
        {
            return Some(slot);
        }
        if matches!(item.name, 45 | 46)
            && self
                .equipment
                .iter()
                .flatten()
                .any(|row| matches!(row.name, 45 | 46))
        {
            return None;
        }
        self.equipment.iter().position(Option::is_none)
    }

    pub fn select(&mut self, category: usize, index: usize) -> bool {
        let Some(item) = self.catalog.item(category, index) else {
            return false;
        };
        if u32::from(item.price) > self.balance {
            return false;
        }
        if !self.catalog.available(category, index) || self.slot(category, item).is_none() {
            self.cart = None;
            return false;
        }
        self.cart = Some(Cart {
            category,
            item: index,
            amount: item.amount.into(),
            spend: item.price.into(),
        });
        true
    }

    pub fn increase(&mut self) -> bool {
        let Some(cart) = &mut self.cart else {
            return false;
        };
        let item = self
            .catalog
            .item(cart.category, cart.item)
            .expect("validated cart");
        if (cart.category == 8 && cart.item >= 3)
            || u64::from(cart.spend) + u64::from(item.price) > u64::from(self.balance)
        {
            return false;
        }
        cart.spend += u32::from(item.price);
        cart.amount = cart.amount.wrapping_add(u32::from(item.amount));
        true
    }

    pub fn decrease(&mut self) -> bool {
        let Some(cart) = &mut self.cart else {
            return false;
        };
        let item = self
            .catalog
            .item(cart.category, cart.item)
            .expect("validated cart");
        if (cart.category == 8 && cart.item >= 3) || cart.amount <= u32::from(item.amount) {
            return false;
        }
        cart.spend -= u32::from(item.price);
        cart.amount -= u32::from(item.amount);
        true
    }

    pub fn cancel(&mut self) {
        self.cart = None;
    }

    pub fn buy(&mut self) -> bool {
        let Some(cart) = self.cart else {
            return false;
        };
        let item = self
            .catalog
            .item(cart.category, cart.item)
            .expect("validated cart");
        let Some(slot) = self.slot(cart.category, item) else {
            return false;
        };
        let row = Some(Owned {
            category: cart.category,
            item: cart.item,
            name: item.name,
            amount: cart.amount as u16,
            paid: cart.spend as u16,
        });
        if cart.category == 8 {
            self.equipment[slot] = row;
        } else {
            self.weapons[slot] = row;
        }
        self.balance -= cart.spend;
        self.cart = None;
        true
    }

    pub fn sell_weapon(&mut self, slot: usize) -> bool {
        let Some(row) = self.weapons.get_mut(slot).and_then(Option::take) else {
            return false;
        };
        self.refund(row);
        true
    }

    pub fn sell_equipment(&mut self, slot: usize) -> bool {
        let Some(row) = self.equipment.get_mut(slot).and_then(Option::take) else {
            return false;
        };
        self.refund(row);
        true
    }

    fn refund(&mut self, row: Owned) {
        self.balance = self.balance.wrapping_add(u32::from(row.paid));
        self.cart = Some(Cart {
            category: row.category,
            item: row.item,
            amount: row.amount.into(),
            spend: row.paid.into(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::armoury::catalog::Mode;
    fn shop(balance: u32) -> Transactions {
        Transactions::new(Catalog::new(Mode::Campaign, 2, [1; 15]).unwrap(), balance)
    }

    #[test]
    fn auto_stops_top_up_after_any_unaffordable_weapon() {
        let mut shop = shop(1200);
        let mut draws = [0, 0, 0, 1, 0, 6, 0].into_iter();
        let (ages, _) = shop.auto(|_| draws.next().expect("exact call order"));
        assert_eq!(&ages[..3], &[5, 7, 6]);
        assert_eq!(shop.balance(), 100);
        // First pass buys Needler ammo, cannot afford Hades, then buys Plasma.
        assert_eq!(shop.weapons()[0].unwrap().name, 6);
        assert_eq!(shop.weapons()[0].unwrap().amount, 600);
        assert_eq!(shop.weapons()[1].unwrap().name, 2);
        assert_eq!(shop.weapons()[1].unwrap().amount, 600);
        assert_eq!(shop.weapons()[2].unwrap().name, 9);
        assert!(draws.next().is_none());
    }

    #[test]
    fn auto_unaffordable_valid_candidate_ends_attempt() {
        let mut shop = shop(100);
        let mut draws = [0, 1, 0, 1, 0, 0, 0].into_iter();
        shop.auto(|_| draws.next().expect("must not retry expensive candidate"));
        assert_eq!(shop.balance(), 0);
        assert_eq!(shop.weapons()[0].unwrap().name, 2);
        assert!(draws.next().is_none());
    }

    #[test]
    fn original_manual_needler_purchase_sell_cancel_sequence() {
        let mut shop = shop(3500);
        assert!(shop.select(0, 0));
        assert_eq!(
            (shop.balance(), shop.cash(), shop.cart().unwrap().amount),
            (3500, 3400, 300)
        );
        for _ in 0..6 {
            assert!(shop.increase());
        }
        assert_eq!((shop.cash(), shop.cart().unwrap().amount), (2800, 2100));
        assert!(shop.buy());
        assert_eq!(shop.balance(), 2800);
        assert_eq!(shop.weapons()[0].unwrap().amount, 2100);
        assert!(shop.sell_weapon(0));
        assert!(!shop.has_weapon());
        assert_eq!((shop.balance(), shop.cash()), (3500, 2800));
        shop.cancel();
        assert_eq!(shop.cash(), 3500);
    }

    #[test]
    fn quantity_floor_budget_and_scanners() {
        let mut shop = shop(250);
        assert!(shop.select(0, 0));
        assert!(!shop.decrease());
        assert!(shop.increase());
        assert!(!shop.increase());
        assert!(shop.decrease());
        assert_eq!(shop.cart().unwrap().amount, 300);
        let mut shop = self::shop(3500);
        assert!(shop.select(8, 3));
        assert!(!shop.increase());
        assert!(!shop.decrease());
        assert!(shop.buy());
        assert!(!shop.has_weapon());
        assert!(!shop.select(8, 4));
        assert!(shop.cart().is_none());
    }

    #[test]
    fn duplicate_weapon_and_equipment_replacement_differ() {
        let mut shop = shop(3500);
        assert!(shop.select(0, 0));
        assert!(shop.buy());
        assert!(!shop.select(0, 0));
        assert!(shop.select(8, 1));
        assert!(shop.increase());
        assert!(shop.buy());
        assert!(shop.select(8, 1));
        assert!(shop.buy());
        assert_eq!(shop.balance(), 2650);
        assert_eq!(shop.equipment()[0].unwrap().amount, 5);
        assert!(shop.sell_equipment(0));
        shop.cancel();
        assert_eq!(shop.balance(), 2900); // Only the replacement's cost is refundable.
    }

    #[test]
    fn unaffordable_selection_preserves_cart_and_entry_floors_money() {
        let mut shop = shop(0);
        assert_eq!(shop.balance(), 100);
        assert!(shop.select(0, 0));
        assert!(!shop.select(0, 1));
        assert_eq!(shop.cart().unwrap().item, 0);
    }
}
