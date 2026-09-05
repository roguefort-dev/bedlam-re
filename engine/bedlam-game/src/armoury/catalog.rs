//! Original armoury catalog, EXW FUN_0044395b.
//! Provenance and availability mapping: docs/RE-EXW-MISSION-ROOM.md.

/// A catalog entry; amount means ammunition or equipment charge as authored.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Item {
    pub name: u16,
    pub price: u16,
    pub amount: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Category {
    pub anchor: (i32, i32),
    pub y_offset: i32,
    pub columns: i32,
    pub rows: i32,
    pub items: &'static [Item],
}

pub const CATEGORIES: [Category; 9] = [
    Category {
        anchor: (237, 97),
        y_offset: 37,
        columns: 26,
        rows: 6,
        items: &[
            Item {
                name: 2,
                price: 100,
                amount: 300,
            },
            Item {
                name: 3,
                price: 250,
                amount: 400,
            },
            Item {
                name: 4,
                price: 400,
                amount: 500,
            },
        ],
    },
    Category {
        anchor: (390, 97),
        y_offset: 37,
        columns: 23,
        rows: 6,
        items: &[
            Item {
                name: 9,
                price: 500,
                amount: 1,
            },
            Item {
                name: 10,
                price: 700,
                amount: 1,
            },
            Item {
                name: 11,
                price: 900,
                amount: 1,
            },
        ],
    },
    Category {
        anchor: (603, 200),
        y_offset: 56,
        columns: 23,
        rows: 7,
        items: &[
            Item {
                name: 37,
                price: 200,
                amount: 24,
            },
            Item {
                name: 38,
                price: 400,
                amount: 36,
            },
            Item {
                name: 39,
                price: 600,
                amount: 72,
            },
            Item {
                name: 40,
                price: 800,
                amount: 144,
            },
        ],
    },
    Category {
        anchor: (397, 364),
        y_offset: 59,
        columns: 26,
        rows: 10,
        items: &[
            Item {
                name: 24,
                price: 250,
                amount: 60,
            },
            Item {
                name: 25,
                price: 350,
                amount: 30,
            },
            Item {
                name: 27,
                price: 50,
                amount: 96,
            },
            Item {
                name: 28,
                price: 100,
                amount: 144,
            },
            Item {
                name: 29,
                price: 100,
                amount: 96,
            },
            Item {
                name: 30,
                price: 200,
                amount: 144,
            },
        ],
    },
    Category {
        anchor: (280, 375),
        y_offset: 62,
        columns: 26,
        rows: 10,
        items: &[
            Item {
                name: 20,
                price: 100,
                amount: 80,
            },
            Item {
                name: 21,
                price: 200,
                amount: 120,
            },
            Item {
                name: 22,
                price: 350,
                amount: 160,
            },
            Item {
                name: 16,
                price: 150,
                amount: 60,
            },
            Item {
                name: 17,
                price: 250,
                amount: 120,
            },
            Item {
                name: 18,
                price: 400,
                amount: 180,
            },
        ],
    },
    Category {
        anchor: (165, 356),
        y_offset: 50,
        columns: 20,
        rows: 3,
        items: &[Item {
            name: 14,
            price: 500,
            amount: 20,
        }],
    },
    Category {
        anchor: (95, 326),
        y_offset: 50,
        columns: 26,
        rows: 6,
        items: &[
            Item {
                name: 6,
                price: 200,
                amount: 300,
            },
            Item {
                name: 7,
                price: 500,
                amount: 600,
            },
            Item {
                name: 8,
                price: 800,
                amount: 900,
            },
        ],
    },
    Category {
        anchor: (46, 269),
        y_offset: 46,
        columns: 23,
        rows: 7,
        items: &[
            Item {
                name: 32,
                price: 200,
                amount: 24,
            },
            Item {
                name: 33,
                price: 350,
                amount: 36,
            },
            Item {
                name: 34,
                price: 700,
                amount: 72,
            },
            Item {
                name: 35,
                price: 950,
                amount: 108,
            },
        ],
    },
    Category {
        anchor: (68, 204),
        y_offset: 46,
        columns: 25,
        rows: 9,
        items: &[
            Item {
                name: 42,
                price: 500,
                amount: 15,
            },
            Item {
                name: 43,
                price: 250,
                amount: 5,
            },
            Item {
                name: 44,
                price: 300,
                amount: 25,
            },
            Item {
                name: 45,
                price: 400,
                amount: 1,
            },
            Item {
                name: 46,
                price: 800,
                amount: 1,
            },
        ],
    },
];

/// Successive dwords at EXW 0x46cd48..0x46cd80 copied by 0x444184..215.
const CAMPAIGN_ITEMS: [(usize, usize); 15] = [
    (3, 0),
    (0, 2),
    (7, 1),
    (2, 0),
    (1, 2),
    (5, 0),
    (4, 4),
    (7, 2),
    (2, 1),
    (2, 2),
    (7, 3),
    (2, 3),
    (4, 5),
    (6, 2),
    (3, 1),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Campaign,
    Multiplayer,
}

/// Availability is supplied by campaign state, never inferred from money.
/// Values 1 and transient 2 are both enabled in the original.
#[derive(Clone, Debug)]
pub struct Catalog {
    available: [[bool; 6]; 9],
}

impl Catalog {
    /// Zone is the original one-based zone number (1..=7).
    pub fn new(mode: Mode, zone: u8, campaign_flags: [u32; 15]) -> Option<Self> {
        if !(1..=7).contains(&zone) {
            return None;
        }
        let mut available = [[true; 6]; 9];
        available[8][4] = mode == Mode::Campaign && (2..=4).contains(&zone);
        if mode == Mode::Multiplayer {
            available[2].fill(false);
            available[8].fill(false);
        } else if zone != 7 {
            for ((category, item), flag) in CAMPAIGN_ITEMS.into_iter().zip(campaign_flags) {
                available[category][item] = flag != 0;
            }
        }
        Some(Self { available })
    }

    pub fn item(&self, category: usize, item: usize) -> Option<&'static Item> {
        CATEGORIES.get(category)?.items.get(item)
    }

    pub fn available(&self, category: usize, item: usize) -> bool {
        self.item(category, item).is_some() && self.available[category][item]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_boot_camp_needler_popup() {
        // Live EXD: #1 100/300, #2 250/400, CLASSIFIED 400/500.
        let catalog = Catalog::new(Mode::Campaign, 1, [0; 15]).unwrap();
        assert_eq!(
            catalog.item(0, 0),
            Some(&Item {
                name: 2,
                price: 100,
                amount: 300
            })
        );
        assert_eq!(
            catalog.item(0, 1),
            Some(&Item {
                name: 3,
                price: 250,
                amount: 400
            })
        );
        assert_eq!(
            catalog.item(0, 2),
            Some(&Item {
                name: 4,
                price: 400,
                amount: 500
            })
        );
        assert!(catalog.available(0, 0));
        assert!(catalog.available(0, 1));
        assert!(!catalog.available(0, 2));
        assert!(!catalog.available(0, 3));
        assert!(!catalog.available(9, 0));
    }

    #[test]
    fn campaign_unlocks_do_not_override_scanner_zone_restriction() {
        for zone in 1..=7 {
            let catalog = Catalog::new(Mode::Campaign, zone, [2; 15]).unwrap();
            assert!(catalog.available(0, 2));
            assert_eq!(catalog.available(8, 4), (2..=4).contains(&zone));
        }
        assert!(Catalog::new(Mode::Campaign, 7, [0; 15])
            .unwrap()
            .available(0, 2));
        assert!(Catalog::new(Mode::Campaign, 0, [0; 15]).is_none());
        assert!(Catalog::new(Mode::Campaign, 8, [0; 15]).is_none());
    }

    #[test]
    fn multiplayer_enables_advanced_weapons_but_excludes_equipment_and_category_two() {
        let catalog = Catalog::new(Mode::Multiplayer, 1, [0; 15]).unwrap();
        assert!(catalog.available(0, 2));
        assert!(catalog.available(6, 2));
        for category in [2, 8] {
            for item in 0..CATEGORIES[category].items.len() {
                assert!(!catalog.available(category, item));
            }
        }
    }
}
