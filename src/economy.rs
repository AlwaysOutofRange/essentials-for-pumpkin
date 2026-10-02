//! /balance /pay /baltop /eco

use pumpkin_plugin_api::command::{CommandSender, ConsumedArgs};

use crate::{
    state::with,
    util::*,
};

pub fn balance(sender: &CommandSender, args: &ConsumedArgs, named: bool) -> Res {
    if named {
        let Some(name) = arg_str(args, "name") else {
            return fail("Usage: /balance [player]");
        };
        let found = with(|st| st.find_by_name(&name).map(|(_, d)| (d.name.clone(), st.money(d.balance))));
        return match found {
            Some((n, b)) => {
                say(sender, &format!("Balance of {n}: {b}"));
                ok()
            }
            None => fail("Unknown player."),
        };
    }
    let p = me(sender)?;
    let (id, name) = (id_of(&p), p.get_name());
    let b = with(|st| {
        let bal = st.pd(&id, &name).balance;
        st.money(bal)
    });
    say(sender, &format!("Balance: {b}"));
    ok()
}

pub fn pay(sender: &CommandSender, args: &ConsumedArgs) -> Res {
    let p = me(sender)?;
    let target = need_player(args, "player")?;
    let Some(amount) = arg_f64(args, "amount") else {
        return fail("Usage: /pay <player> <amount>");
    };
    if !amount.is_finite() || amount <= 0.0 {
        return fail("Amount must be greater than zero.");
    }
    let (from_id, from_name) = (id_of(&p), p.get_name());
    let (to_id, to_name) = (id_of(&target), target.get_name());
    if from_id == to_id {
        return fail("You can't pay yourself.");
    }
    let result = with(|st| {
        if st.pd(&from_id, &from_name).balance < amount {
            return Err(());
        }
        st.pd(&from_id, &from_name).balance -= amount;
        st.pd(&to_id, &to_name).balance += amount;
        st.save();
        Ok(st.money(amount))
    });
    match result {
        Ok(m) => {
            say(sender, &format!("Paid {m} to {to_name}."));
            tell(&target, &format!("You received {m} from {from_name}."));
            ok()
        }
        Err(()) => fail("You don't have enough money."),
    }
}

pub fn baltop(sender: &CommandSender) -> Res {
    let lines: Vec<String> = with(|st| {
        let mut v: Vec<(&str, f64)> = st
            .data
            .players
            .values()
            .map(|d| (d.name.as_str(), d.balance))
            .collect();
        v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        v.into_iter()
            .take(10)
            .enumerate()
            .map(|(i, (n, b))| format!("{}. {n}: {}", i + 1, st.money(b)))
            .collect()
    });
    say(sender, "Top balances:");
    for l in lines {
        say(sender, &l);
    }
    ok()
}

#[derive(Clone, Copy)]
pub enum EcoOp {
    Give,
    Take,
    Set,
    Reset,
}

pub fn eco(sender: &CommandSender, args: &ConsumedArgs, op: EcoOp) -> Res {
    let t = need_player(args, "player")?;
    let (id, name) = (id_of(&t), t.get_name());
    let amount = match op {
        EcoOp::Reset => 0.0,
        _ => match arg_f64(args, "amount") {
            Some(a) if a.is_finite() && a >= 0.0 => a,
            _ => return fail("Amount must be a non-negative number."),
        },
    };
    let shown = with(|st| {
        let start = st.config.starting_balance;
        let d = st.pd(&id, &name);
        d.balance = match op {
            EcoOp::Give => d.balance + amount,
            EcoOp::Take => (d.balance - amount).max(0.0),
            EcoOp::Set => amount,
            EcoOp::Reset => start,
        };
        let b = d.balance;
        st.save();
        st.money(b)
    });
    say(sender, &format!("{name}'s balance is now {shown}."));
    ok()
}
