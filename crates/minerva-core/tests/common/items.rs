//! Made-up item data in OSMS Data Explorer's exact JSON shape (ADR 0010): invented items,
//! scrolls and Cash Shop items, written out as the two item files the shell fetches. The
//! names are the game's, as players see them; every figure is made up. Nothing from OSMS
//! is committed.

use minerva_core::osms_items::ItemFiles;
use serde_json::{Map as Object, Value, json};

/// An item in `items.json`'s `items`.
#[derive(Debug, Clone)]
pub struct Item {
    pub id: u64,
    pub name: String,
    /// `category`: "Equipment", "Consumable", "Etc" or "Setup".
    pub category: String,
    /// `sub_category`: "Hat", "Weapon", "Consume", "Etc", "Install"...
    pub sub_category: String,
    /// `weapon_type`, on weapons ("1H Sword", "Claw").
    pub weapon_type: Option<String>,
    /// `stats`, as written.
    pub stats: Object<String, Value>,
    /// Equipment's top-level `price`, as written (OSMS once wrote one as a string).
    pub equip_price: Option<Value>,
}

fn item(id: u64, name: &str, category: &str, sub_category: &str) -> Item {
    Item {
        id,
        name: name.into(),
        category: category.into(),
        sub_category: sub_category.into(),
        weapon_type: None,
        stats: Object::new(),
        equip_price: None,
    }
}

/// A potion, a throwing star or another item used from the Use tab.
pub fn use_item(id: u64, name: &str) -> Item {
    item(id, name, "Consumable", "Consume")
}

/// An ore, a mob's drop or another Etc item.
pub fn etc(id: u64, name: &str) -> Item {
    item(id, name, "Etc", "Etc")
}

/// A chair or another Setup item.
pub fn setup(id: u64, name: &str) -> Item {
    item(id, name, "Setup", "Install")
}

/// Equipment that isn't a weapon: `sub_category` "Hat", "Overall", "Earring"...
pub fn equip(id: u64, name: &str, sub_category: &str) -> Item {
    item(id, name, "Equipment", sub_category)
}

/// A weapon of `weapon_type` ("1H Sword", "Claw").
pub fn weapon(id: u64, name: &str, weapon_type: &str) -> Item {
    Item { weapon_type: Some(weapon_type.into()), ..item(id, name, "Equipment", "Weapon") }
}

impl Item {
    /// What a shop pays for it: top-level `price` on equipment, `stats.price` on the rest.
    pub fn sells_for(self, mesos: u64) -> Self {
        self.priced(json!(mesos))
    }

    /// Equipment's price written as a string, as OSMS once wrote Pink Jester's.
    pub fn sells_for_text(self, mesos: &str) -> Self {
        self.priced(json!(mesos))
    }

    fn priced(mut self, price: Value) -> Self {
        if self.category == "Equipment" {
            self.equip_price = Some(price);
        } else {
            self.stats.insert("price".into(), price);
        }
        self
    }

    /// A shop won't buy it (`notSale`), and OSMS gives it the placeholder price of 1.
    pub fn not_for_sale(mut self) -> Self {
        self.stats.insert("notSale".into(), json!(1));
        self.priced(json!(1))
    }

    /// Recharged at a shop for `unit` mesos per star or bullet (`unitPrice`).
    pub fn recharged_at(mut self, unit: f64) -> Self {
        self.stats.insert("unitPrice".into(), json!(unit));
        self
    }

    fn json(&self) -> Value {
        let mut item = Object::new();
        item.insert("id".into(), json!(self.id));
        item.insert("name".into(), json!(self.name));
        item.insert("category".into(), json!(self.category));
        item.insert("sub_category".into(), json!(self.sub_category));
        item.insert("stats".into(), Value::Object(self.stats.clone()));
        if self.category == "Equipment" {
            item.insert("req_job_label".into(), json!("All"));
        } else {
            item.insert("description".into(), json!(format!("A made-up {}.", self.name)));
        }
        if let Some(price) = &self.equip_price {
            item.insert("price".into(), price.clone());
        }
        if let Some(weapon_type) = &self.weapon_type {
            item.insert("weapon_type".into(), json!(weapon_type));
            item.insert("attack_speed_label".into(), json!("Fast"));
        }
        Value::Object(item)
    }
}

/// A scroll in `items.json`'s `scrolls`: no price, no category; its tier ("Lesser",
/// "Intermediate", "Greater" or "Chaos"), Lesser unless said.
#[derive(Debug, Clone)]
pub struct Scroll {
    pub id: u64,
    pub name: String,
    pub tier: String,
}

pub fn scroll(id: u64, name: &str) -> Scroll {
    Scroll { id, name: name.into(), tier: "Lesser".into() }
}

impl Scroll {
    pub fn of_tier(self, tier: &str) -> Self {
        Scroll { tier: tier.into(), ..self }
    }
}

/// A Cash Shop item in `cash_shop.json`.
#[derive(Debug, Clone)]
pub struct CashItem {
    pub id: u64,
    pub name: String,
    /// Its Cash Shop category ("Equipment", "Weather Effect").
    pub category: String,
}

pub fn cash(id: u64, name: &str, category: &str) -> CashItem {
    CashItem { id, name: name.into(), category: category.into() }
}

/// Made-up item data: what `items.json` and `cash_shop.json` hold.
#[derive(Debug, Clone, Default)]
pub struct ItemWorld {
    pub items: Vec<Item>,
    pub scrolls: Vec<Scroll>,
    pub cash: Vec<CashItem>,
}

pub fn item_world(items: Vec<Item>) -> ItemWorld {
    ItemWorld { items, ..ItemWorld::default() }
}

impl ItemWorld {
    pub fn with(mut self, item: Item) -> Self {
        self.items.push(item);
        self
    }

    pub fn with_scroll(mut self, scroll: Scroll) -> Self {
        self.scrolls.push(scroll);
        self
    }

    pub fn with_cash(mut self, item: CashItem) -> Self {
        self.cash.push(item);
        self
    }

    /// The data as OSMS's two item files.
    pub fn files(&self) -> ItemFiles {
        ItemFiles { items: self.items_json().to_string(), cash_shop: self.cash_shop_json().to_string() }
    }

    fn items_json(&self) -> Value {
        let count = |category: &str| self.items.iter().filter(|item| item.category == category).count();
        let scrolls: Vec<Value> = self
            .scrolls
            .iter()
            .map(|scroll| {
                json!({"id": scroll.id, "name": scroll.name, "equip_slot": "Other", "stat_type": "Attack",
                       "tier": scroll.tier, "description": "Improves something.\nSuccess rate: 100%"})
            })
            .collect();
        json!({
            "items": self.items.iter().map(Item::json).collect::<Vec<_>>(),
            "scrolls": scrolls,
            "counts": {"equipment": count("Equipment"), "consumables": count("Consumable"), "etc": count("Etc"),
                       "setup": count("Setup"), "scrolls": self.scrolls.len()},
            "item_total": self.items.len(),
            "scroll_total": self.scrolls.len(),
            "equipment_meta": {"armor_order": [], "job_filters": [], "weapon_type_order": [], "weapon_types_by_class": {}},
        })
    }

    fn cash_shop_json(&self) -> Value {
        let mut categories: Vec<(String, Vec<Value>)> = Vec::new();
        for item in &self.cash {
            let row = json!({"id": item.id, "name": item.name, "category": item.category, "sub_category": null,
                             "price": 1000, "count": 1, "period": 90, "on_sale": true,
                             "description": "A made-up Cash item.", "thumbnail": format!("images/items/{:08}.png", item.id)});
            match categories.iter_mut().find(|(name, _)| *name == item.category) {
                Some((_, items)) => items.push(row),
                None => categories.push((item.category.clone(), vec![row])),
            }
        }
        let categories: Vec<Value> =
            categories.into_iter().map(|(category, items)| json!({"category": category, "items": items})).collect();
        json!({"categories": categories, "total": self.cash.len()})
    }
}
