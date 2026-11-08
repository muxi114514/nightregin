//! 暗痕与为王之证：PlayerGameData 内的 u32（2026-10-27 按数值定位，各只有一处；暗痕写入后商店显示与读档均正确）。

use super::memory::{read, write};

const MURK: usize = 0xD0;
const SIGILS: usize = 0x5C;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Currency {
    Murk,
    Sigils,
}

impl Currency {
    fn offset(self) -> usize {
        match self {
            Self::Murk => MURK,
            Self::Sigils => SIGILS,
        }
    }
}

pub fn balance(game_data: usize, currency: Currency) -> Option<u32> {
    read::<u32>(game_data + currency.offset())
}

/// 扣除 amount，返回剩余；余额不足或读写失败时不改动
pub fn spend(game_data: usize, currency: Currency, amount: u32) -> Result<u32, String> {
    let have = balance(game_data, currency).ok_or("cannot read balance")?;
    let left = have.checked_sub(amount).ok_or(format!("balance {have} < {amount}"))?;
    if write(game_data + currency.offset(), left) { Ok(left) } else { Err("write failed".into()) }
}

/// 退还（扣费后写遗物失败时用）
pub fn refund(game_data: usize, currency: Currency, amount: u32) -> bool {
    balance(game_data, currency).is_some_and(|have| write(game_data + currency.offset(), have.saturating_add(amount)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spends_and_refunds() {
        let mut pgd = [0u32; 0x40];
        pgd[MURK / 4] = 5466767;
        pgd[SIGILS / 4] = 1344;
        let gd = pgd.as_mut_ptr() as usize;
        assert_eq!(spend(gd, Currency::Murk, 600), Ok(5466167));
        assert!(spend(gd, Currency::Sigils, 2000).is_err());
        assert_eq!(balance(gd, Currency::Sigils), Some(1344));
        assert!(refund(gd, Currency::Murk, 600));
        assert_eq!(pgd[MURK / 4], 5466767);
    }
}
