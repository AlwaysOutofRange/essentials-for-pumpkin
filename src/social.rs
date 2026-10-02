//! Chat and information commands.

use pumpkin_plugin_api::{
    Server,
    command::{CommandSender, ConsumedArgs},
};

use crate::{
    state::{now, with},
    util::*,
};

pub fn msg(sender: &CommandSender, args: &ConsumedArgs) -> Res {
    let target = need_player(args, "target")?;
    let Some(body) = arg_str(args, "message") else {
        return fail("Usage: /msg <player> <message>");
    };
    deliver(sender, &target, &body)
}

pub fn reply(sender: &CommandSender, server: &Server, args: &ConsumedArgs) -> Res {
    let p = me(sender)?;
    let Some(body) = arg_str(args, "message") else {
        return fail("Usage: /r <message>");
    };
    let id = id_of(&p);
    let Some(other_id) = with(|st| st.last_msg.get(&id).cloned()) else {
        return fail("You have nobody to reply to.");
    };
    let Some(target) = lookup(server, &other_id) else {
        return fail("That player is no longer online.");
    };
    deliver(sender, &target, &body)
}

fn deliver(sender: &CommandSender, target: &pumpkin_plugin_api::Player, body: &str) -> Res {
    if let Some(p) = sender.as_player() {
        let id = id_of(&p);
        let muted = with(|st| st.data.players.get(&id).is_some_and(|d| d.muted));
        if muted {
            return fail("You are muted.");
        }
    }
    let from = sender.get_name();
    sender.send_message(text(&format!("&7[me -> {}] &r{body}", target.get_name())));
    tell_fmt(target, &format!("&7[{from} -> me] &r{body}"));
    if let Some(p) = sender.as_player() {
        let (a, b) = (id_of(&p), id_of(target));
        with(|st| {
            st.last_msg.insert(a.clone(), b.clone());
            st.last_msg.insert(b, a);
        });
    }
    ok()
}

pub fn action(sender: &CommandSender, server: &Server, args: &ConsumedArgs) -> Res {
    let Some(body) = arg_str(args, "action") else {
        return fail("Usage: /me <action>");
    };
    if let Some(p) = sender.as_player() {
        let id = id_of(&p);
        if with(|st| st.data.players.get(&id).is_some_and(|d| d.muted)) {
            return fail("You are muted.");
        }
    }
    broadcast(server, &format!("* {} {body}", sender.get_name()));
    ok()
}

pub fn announce(server: &Server, args: &ConsumedArgs) -> Res {
    let Some(body) = arg_str(args, "message") else {
        return fail("Usage: /broadcast <message>");
    };
    broadcast(server, &format!("&6[&4Broadcast&6] &a{body}"));
    ok()
}

pub fn nick(sender: &CommandSender, args: &ConsumedArgs, other: Option<&str>) -> Res {
    let p = target(sender, args, other)?;
    let Some(wanted) = arg_str(args, "nick") else {
        return fail("Usage: /nick [player] <nickname|off>");
    };
    let id = id_of(&p);
    let name = p.get_name();
    if wanted.eq_ignore_ascii_case("off") {
        with(|st| {
            st.pd(&id, &name).nick = None;
            st.save();
        });
        p.set_display_name(text(&name));
        say(sender, &format!("Nickname of {name} removed."));
    } else {
        let prefix = with(|st| st.config.nick_prefix.clone());
        with(|st| {
            st.pd(&id, &name).nick = Some(wanted.clone());
            st.save();
        });
        p.set_display_name(text(&format!("{prefix}{wanted}")));
        say(sender, &format!("Nickname of {name} set to {wanted}."));
    }
    ok()
}

/// Toggle mute on a player.
pub fn mute(sender: &CommandSender, args: &ConsumedArgs) -> Res {
    let t = need_player(args, "player")?;
    let (id, name) = (id_of(&t), t.get_name());
    let muted = with(|st| {
        let d = st.pd(&id, &name);
        d.muted = !d.muted;
        let m = d.muted;
        st.save();
        m
    });
    if muted {
        tell(&t, "You have been muted.");
        say(sender, &format!("{name} is now muted."));
    } else {
        tell(&t, "You have been unmuted.");
        say(sender, &format!("{name} is no longer muted."));
    }
    ok()
}

fn send_lines(sender: &CommandSender, lines: Vec<String>, who: &str) {
    for l in lines {
        sender.send_message(text(&l.replace("{player}", who)));
    }
}

pub fn motd(sender: &CommandSender) -> Res {
    let lines = with(|st| st.config.motd.clone());
    send_lines(sender, lines, &sender.get_name());
    ok()
}

pub fn rules(sender: &CommandSender) -> Res {
    let lines = with(|st| st.config.rules.clone());
    send_lines(sender, lines, &sender.get_name());
    ok()
}

pub fn list(sender: &CommandSender, server: &Server) -> Res {
    let hidden: Vec<String> = with(|st| {
        st.data
            .players
            .iter()
            .filter(|(_, d)| d.vanished)
            .map(|(id, _)| id.clone())
            .collect()
    });
    let names: Vec<String> = server
        .get_all_players()
        .iter()
        .filter(|p| !hidden.contains(&id_of(p)))
        .map(|p| p.get_name())
        .collect();
    say(
        sender,
        &format!(
            "Online players ({}/{}): {}",
            names.len(),
            server.get_max_players(),
            names.join(", ")
        ),
    );
    ok()
}

pub fn getpos(sender: &CommandSender, args: &ConsumedArgs, other: Option<&str>) -> Res {
    let p = target(sender, args, other)?;
    let l = loc_of(&p);
    say(
        sender,
        &format!(
            "{} is at {:.1}, {:.1}, {:.1} in '{}' (yaw {:.0}, pitch {:.0}).",
            p.get_name(),
            l.x,
            l.y,
            l.z,
            l.world,
            l.yaw,
            l.pitch
        ),
    );
    ok()
}

pub fn ping(sender: &CommandSender) -> Res {
    match sender.as_player() {
        Some(p) => say(sender, &format!("Pong! {} ms", p.get_ping())),
        None => say(sender, "Pong!"),
    }
    ok()
}

pub fn near(sender: &CommandSender, server: &Server, args: &ConsumedArgs, has_radius: bool) -> Res {
    let p = me(sender)?;
    let radius = if has_radius { arg_f64(args, "radius").unwrap_or(200.0) } else { 200.0 };
    let (px, py, pz) = p.get_position();
    let world = p.get_world().get_name();
    let my_id = id_of(&p);
    let mut found = Vec::new();
    for o in server.get_all_players() {
        if id_of(&o) == my_id || o.get_world().get_name() != world {
            continue;
        }
        let (x, y, z) = o.get_position();
        let d = ((x - px).powi(2) + (y - py).powi(2) + (z - pz).powi(2)).sqrt();
        if d <= radius {
            found.push(format!("{} ({}m)", o.get_name(), d.round() as i64));
        }
    }
    if found.is_empty() {
        say(sender, "No players nearby.");
    } else {
        say(sender, &format!("Nearby: {}", found.join(", ")));
    }
    ok()
}

pub fn seen(sender: &CommandSender, server: &Server, args: &ConsumedArgs) -> Res {
    let Some(name) = arg_str(args, "name") else {
        return fail("Usage: /seen <player>");
    };
    if let Some(p) = server.get_player_by_name(&name) {
        say(sender, &format!("{} is currently online.", p.get_name()));
        return ok();
    }
    let seen = with(|st| st.find_by_name(&name).map(|(_, d)| (d.name.clone(), d.last_seen)));
    match seen {
        Some((n, ts)) if ts > 0 => {
            say(sender, &format!("{n} was last seen {} ago.", ago(now().saturating_sub(ts))));
            ok()
        }
        _ => fail("That player has never joined this server."),
    }
}

fn ago(secs: u64) -> String {
    match secs {
        0..=59 => format!("{secs}s"),
        60..=3599 => format!("{}m", secs / 60),
        3600..=86399 => format!("{}h {}m", secs / 3600, (secs % 3600) / 60),
        _ => format!("{}d {}h", secs / 86400, (secs % 86400) / 3600),
    }
}

pub fn afk(sender: &CommandSender, server: &Server) -> Res {
    let p = me(sender)?;
    let id = id_of(&p);
    let now_afk = with(|st| {
        if st.afk.remove(&id) {
            false
        } else {
            st.afk.insert(id.clone());
            true
        }
    });
    let word = if now_afk { "now" } else { "no longer" };
    broadcast(server, &format!("&7* {} is {word} AFK.", p.get_name()));
    ok()
}
