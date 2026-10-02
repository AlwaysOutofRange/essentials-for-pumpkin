//! Player-state commands: heal, feed, fly, god, speed, vanish, ...

use pumpkin_plugin_api::{
    Server,
    command::{CommandSender, ConsumedArgs},
    player::PlayerWeather,
};

use crate::{
    state::with,
    util::*,
};

pub fn heal(sender: &CommandSender, args: &ConsumedArgs, other: Option<&str>) -> Res {
    let p = target(sender, args, other)?;
    p.set_health(p.get_max_health());
    p.set_food_level(20);
    p.set_saturation(20.0);
    p.clear_effects();
    tell(&p, "You have been healed.");
    if other.is_some() {
        say(sender, &format!("Healed {}.", p.get_name()));
    }
    ok()
}

pub fn feed(sender: &CommandSender, args: &ConsumedArgs, other: Option<&str>) -> Res {
    let p = target(sender, args, other)?;
    p.set_food_level(20);
    p.set_saturation(20.0);
    tell(&p, "Your appetite has been sated.");
    if other.is_some() {
        say(sender, &format!("Fed {}.", p.get_name()));
    }
    ok()
}

pub fn fly(sender: &CommandSender, args: &ConsumedArgs, other: Option<&str>) -> Res {
    let p = target(sender, args, other)?;
    let allow = !p.get_abilities().allow_flying;
    p.set_allow_flight(allow);
    if !allow {
        p.set_flying(false);
    }
    let state = if allow { "enabled" } else { "disabled" };
    tell(&p, &format!("Flight {state}."));
    if other.is_some() {
        say(sender, &format!("Flight {state} for {}.", p.get_name()));
    }
    ok()
}

pub fn god(sender: &CommandSender, args: &ConsumedArgs, other: Option<&str>) -> Res {
    let p = target(sender, args, other)?;
    let on = !p.get_abilities().invulnerable;
    p.set_invulnerable(on);
    let state = if on { "enabled" } else { "disabled" };
    tell(&p, &format!("God mode {state}."));
    if other.is_some() {
        say(sender, &format!("God mode {state} for {}.", p.get_name()));
    }
    ok()
}

/// EssentialsX speed scale (1-10, 1 = vanilla) to Pumpkin's protocol scale.
/// Essentials works on Bukkit's scale (fly 0.1 / walk 0.2 by default), which
/// is exactly twice the protocol value.
fn real_speed(n: f64, fly: bool) -> f32 {
    let default = if fly { 0.1 } else { 0.2 };
    let n = n.clamp(0.0, 10.0);
    let bukkit = if n < 1.0 {
        default * n
    } else {
        default + (n - 1.0) / 9.0 * (1.0 - default)
    };
    (bukkit / 2.0) as f32
}

/// `kind`: `Some(true)` = fly, `Some(false)` = walk, `None` = whichever the player is doing.
pub fn speed(sender: &CommandSender, args: &ConsumedArgs, kind: Option<bool>) -> Res {
    let p = me(sender)?;
    let Some(n) = arg_f64(args, "speed") else {
        return fail("Usage: /speed [fly|walk] <1-10>");
    };
    if !(0.0..=10.0).contains(&n) {
        return fail("Speed must be between 0 and 10.");
    }
    let fly = kind.unwrap_or_else(|| p.is_flying());
    if fly {
        p.set_fly_speed(real_speed(n, true));
    } else {
        p.set_walk_speed(real_speed(n, false));
    }
    say(sender, &format!("{} speed set to {n}.", if fly { "Fly" } else { "Walk" }));
    ok()
}

pub fn vanish(sender: &CommandSender, server: &Server) -> Res {
    let p = me(sender)?;
    let (id, name) = (id_of(&p), p.get_name());
    let on = with(|st| {
        let d = st.pd(&id, &name);
        d.vanished = !d.vanished;
        let v = d.vanished;
        st.save();
        v
    });
    let uid = p.get_id();
    for other in server.get_all_players() {
        if id_of(&other) == id {
            continue;
        }
        if let Some(handle) = server.get_player_by_uuid(dup(&uid)) {
            if on {
                other.hide_player(handle);
            } else {
                other.show_player(handle);
            }
        }
    }
    say(sender, if on { "You are now vanished." } else { "You are visible again." });
    ok()
}

pub fn suicide(sender: &CommandSender, server: &Server) -> Res {
    let p = me(sender)?;
    broadcast(server, &format!("&7{} took their own life... in the game.", p.get_name()));
    p.kill();
    ok()
}

pub fn enderchest(sender: &CommandSender) -> Res {
    let p = me(sender)?;
    p.open_ender_chest();
    ok()
}

pub fn ptime(sender: &CommandSender, args: &ConsumedArgs) -> Res {
    let p = me(sender)?;
    let Some(v) = arg_str(args, "time") else {
        return fail("Usage: /ptime <day|noon|night|midnight|reset|ticks>");
    };
    let ticks: u64 = match v.to_lowercase().as_str() {
        "reset" => {
            p.reset_player_time();
            say(sender, "Your time now follows the world.");
            return ok();
        }
        "dawn" | "sunrise" => 23000,
        "day" => 1000,
        "noon" => 6000,
        "dusk" | "sunset" => 12000,
        "night" => 13000,
        "midnight" => 18000,
        other => match other.trim_end_matches("ticks").parse::<u64>() {
            Ok(t) => t,
            Err(_) => return fail("Unknown time. Try day, noon, night, midnight, reset or a tick count."),
        },
    };
    p.set_player_time(ticks, false);
    say(sender, &format!("Your time is now fixed at {ticks} ticks."));
    ok()
}

pub fn pweather(sender: &CommandSender, args: &ConsumedArgs) -> Res {
    let p = me(sender)?;
    let Some(v) = arg_str(args, "weather") else {
        return fail("Usage: /pweather <sun|storm|reset>");
    };
    match v.to_lowercase().as_str() {
        "reset" => {
            p.reset_player_weather();
            say(sender, "Your weather now follows the world.");
        }
        "sun" | "clear" => {
            p.set_player_weather(PlayerWeather::Clear);
            say(sender, "Your weather is now clear.");
        }
        "storm" | "rain" | "downfall" => {
            p.set_player_weather(PlayerWeather::Downfall);
            say(sender, "Your weather is now stormy.");
        }
        _ => return fail("Unknown weather. Try sun, storm or reset."),
    }
    ok()
}
