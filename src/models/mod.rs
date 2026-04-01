// Моделі даних — Rust структури, що відповідають таблицям БД
pub mod counterparty;
pub mod act;

pub use counterparty::{Counterparty, NewCounterparty, UpdateCounterparty};
#[allow(unused_imports)]
pub use act::{Act, ActItem, ActListRow, ActStatus, NewAct, NewActItem, UpdateAct};
